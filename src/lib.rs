use std::{cell::RefCell, collections::HashMap, rc::Rc};

use mupdf::{
    page::StextPage, ColorParams, Colorspace, Device, Image, ImageFormat, Matrix, NativeDevice,
    Pixmap, TextPageFlags,
};
use pyo3::{
    exceptions::{PyIOError, PyValueError},
    prelude::*,
    types::PyBytes,
};

#[cfg(any(
    target_os = "linux",
    target_os = "freebsd",
    target_os = "openbsd",
    target_os = "netbsd"
))]
fn ensure_writable_dir(dir: &std::path::Path) -> bool {
    // Cache directory may not exist in containers/serverless runtimes.
    if std::fs::create_dir_all(dir).is_err() {
        return false;
    }

    let test_file = dir.join(format!(
        ".fast-pdf-extract-write-test-{}",
        std::process::id()
    ));
    // Permissions can allow the directory to exist but still block writes.
    if std::fs::write(&test_file, b"").is_err() {
        return false;
    }

    let _ = std::fs::remove_file(test_file);
    true
}

static FONTCONFIG_CACHE_ONCE: std::sync::Once = std::sync::Once::new();

fn ensure_fontconfig_cache_dir() {
    FONTCONFIG_CACHE_ONCE.call_once(|| {
        #[cfg(any(
            target_os = "linux",
            target_os = "freebsd",
            target_os = "openbsd",
            target_os = "netbsd"
        ))]
        {
            let current_cache = std::env::var_os("XDG_CACHE_HOME")
                .map(std::path::PathBuf::from)
                .or_else(|| {
                    std::env::var_os("HOME")
                        .map(std::path::PathBuf::from)
                        .map(|home| home.join(".cache"))
                });

            // Fontconfig needs a writable cache when resolving non-embedded fonts.
            if current_cache
                .as_deref()
                .is_some_and(|dir| ensure_writable_dir(&dir.join("fontconfig")))
            {
                return;
            }

            let fallback_cache =
                std::env::temp_dir().join(format!("fast-pdf-extract-cache-{}", std::process::id()));

            // If even temp is not writable, leave the environment unchanged.
            if ensure_writable_dir(&fallback_cache.join("fontconfig")) {
                std::env::set_var("XDG_CACHE_HOME", fallback_cache);
            }
        }
    });
}

fn to_pyerr<E: ToString>(err: E) -> PyErr {
    PyValueError::new_err(err.to_string())
}

type Pages = Vec<Vec<String>>;

fn get_styled_paragraphs(stext_page: StextPage) -> Vec<String> {
    let mut sizes = stext_page
        .blocks
        .iter()
        .filter(|block| block.r#type == "text")
        .map(|block| block.lines.iter().map(|line| line.font.size))
        .flatten()
        .collect::<Vec<u32>>();
    sizes.sort();
    let base_size = if sizes.len() > 0 {
        sizes[sizes.len() / 2]
    } else {
        100
    };

    stext_page
        .blocks
        .iter()
        .map(|block| {
            let mut lines: Vec<String> = vec![];
            let mut all_large = true;
            for line in &block.lines {
                if line.font.size <= base_size && line.font.weight != "bold" {
                    all_large = false;
                };
                lines.push(line.text.clone());
            }
            let joined = lines.join("\n");
            if joined.trim() != "" && all_large {
                format!("**{}**", joined.trim())
            } else {
                joined
            }
        })
        .collect::<Vec<String>>()
}

fn get_hash(text: &str) -> String {
    text.chars()
        .filter(|c| !c.is_whitespace() && !c.is_numeric())
        .collect()
}

enum Position {
    Top,
    Bottom,
}

fn get_common_hash(pages: &Pages, mincount: u32, position: Position) -> Option<String> {
    let mut counts = HashMap::new();
    for paragraphs in pages {
        let paragraph = match position {
            Position::Top => paragraphs.first(),
            Position::Bottom => paragraphs.last(),
        };
        if let Some(text) = paragraph {
            let hash = get_hash(text);
            *counts.entry(hash).or_insert(0) += 1;
        };
    }

    for (hash, count) in counts {
        if count >= mincount {
            return Some(hash);
        }
    }
    Option::None
}

fn remove_headers_footers(pages: &mut Pages) {
    let mincount = match pages.len() {
        n if n <= 2 => 2.0,
        n if n <= 50 => 2.0 / 3.0 * (pages.len() as f64),
        _ => (pages.len() as f64) * 0.4,
    } as u32;

    while let Some(hash) = get_common_hash(&pages, mincount, Position::Top) {
        for paragraphs in &mut *pages {
            if let Some(paragraph) = &paragraphs.first() {
                if hash == get_hash(paragraph) {
                    paragraphs.remove(0);
                }
            }
        }
    }

    while let Some(hash) = get_common_hash(&pages, mincount, Position::Bottom) {
        for paragraphs in &mut *pages {
            if let Some(paragraph) = &paragraphs.last() {
                if hash == get_hash(paragraph) {
                    paragraphs.pop();
                }
            }
        }
    }
}

/// a word is non-english if it has multiple ascii characters
/// or looks mixed case
fn is_english_word(word: &str) -> bool {
    let mut had_lower = false;
    let mut non_ascii_count = 0;
    let allowed = ['‘', '’', '‚', '‛', '“', '”', '„', '‟', '–'];

    for c in word.chars() {
        had_lower = had_lower || c.is_lowercase();
        if !c.is_ascii() && !allowed.contains(&c) {
            non_ascii_count += 1
        };

        // mixed case
        if had_lower && c.is_uppercase() {
            return false;
        }

        // is non-ascii
        if non_ascii_count > 1 {
            return false;
        }
    }
    true
}

fn is_non_english(text: &str) -> bool {
    let mut bad_words_count = 0;
    let mut original_count = 0;

    for word in text.split_whitespace() {
        original_count += 1;
        if !is_english_word(word) {
            bad_words_count += 1
        };
    }

    let bad_char_count = text.chars().filter(|c| !c.is_ascii()).count();
    let char_count = text.chars().count();

    (bad_words_count > 4 && bad_words_count as f64 > original_count as f64 * 0.1)
        || (char_count > 5 && bad_char_count > (char_count / 2))
}

fn remove_non_english(pages: Pages) -> Pages {
    let mut before_len = 0;
    let mut after_len = 0;
    let mut after_pages: Pages = Pages::new();

    for paragraphs in &pages {
        let mut after_page: Vec<String> = vec![];
        for paragraph in paragraphs {
            before_len += paragraph.len();
            if !is_non_english(&paragraph) {
                after_page.push(paragraph.to_string());
                after_len += paragraph.len();
            }
        }
        // Preserve every page so extracted pages align with the source PDF.
        after_pages.push(after_page);
    }

    if (after_len as f64) < (0.9 * before_len as f64) {
        after_pages
    } else {
        pages
    }
}

#[pyfunction]
fn get_page_count(filename: String) -> PyResult<i32> {
    let document = mupdf::pdf::document::PdfDocument::open(&filename)
        .map_err(|err| PyIOError::new_err(err.to_string()))?;
    document.page_count().map_err(to_pyerr)
}

#[pyfunction]
fn get_pages(filename: String) -> PyResult<Vec<String>> {
    ensure_fontconfig_cache_dir();

    let document = mupdf::pdf::document::PdfDocument::open(&filename)
        .map_err(|err| PyIOError::new_err(err.to_string()))?;
    let mut pages = document
        .pages()
        .map_err(to_pyerr)?
        .map(|page| {
            // Page extraction can fail for malformed PDFs. Keep an empty page
            // instead of shifting all later page indexes.
            let Ok(page) = page else {
                return vec![];
            };
            let Ok(text_page) = page.to_text_page(
                TextPageFlags::COLLECT_STYLES | TextPageFlags::USE_GID_FOR_UNKNOWN_UNICODE,
            ) else {
                return vec![];
            };
            let Ok(stext_json) = text_page.to_json(1.0) else {
                return vec![];
            };
            let Ok(stext_page) = serde_json::from_str(stext_json.as_str()) else {
                return vec![];
            };
            get_styled_paragraphs(stext_page)
        })
        .collect::<Pages>();

    remove_headers_footers(&mut pages);
    let pages = remove_non_english(pages);
    Ok(pages
        .iter()
        .map(|paragraphs| paragraphs.join("\n\n"))
        .collect())
}

// Collect actual image drawing operations; structured text also rasterizes gradients.
#[derive(Default)]
struct ImageCollector(Vec<(Image, Matrix)>);

impl NativeDevice for ImageCollector {
    fn fill_image(&mut self, image: &Image, matrix: Matrix, _: f32, _: ColorParams) {
        self.0.push((image.clone(), matrix));
    }

    fn fill_image_mask(
        &mut self,
        image: &Image,
        matrix: Matrix,
        _: &Colorspace,
        _: &[f32],
        _: f32,
        _: ColorParams,
    ) {
        self.0.push((image.clone(), matrix));
    }
}

fn image_to_png(image: &Image) -> Result<Vec<u8>, mupdf::Error> {
    // Draw at native resolution to convert CMYK and preserve soft masks.
    let mut pixmap = Pixmap::new_with_w_h(
        &Colorspace::device_rgb(),
        image.width() as i32,
        image.height() as i32,
        true,
    )?;
    pixmap.clear()?;
    let device = Device::from_pixmap(&pixmap)?;
    let matrix = Matrix::new_scale(image.width() as f32, image.height() as f32);

    // Drawing an image alone does not apply its PDF soft/explicit mask.
    let mask = image.mask();
    if let Some(mask) = &mask {
        device.clip_image_mask(mask, &matrix)?;
    }

    // Stencil masks have no intrinsic color; export them in black.
    let result = if image.color_space().is_none() {
        device.fill_image_mask(
            image,
            &matrix,
            &Colorspace::device_rgb(),
            &[0.0, 0.0, 0.0],
            1.0,
            Default::default(),
        )
    } else {
        device.fill_image(image, &matrix, 1.0, Default::default())
    };
    // Balance the clip even when decoding fails, before closing the device.
    if mask.is_some() {
        device.pop_clip()?;
    }
    result?;
    drop(device);

    let mut bytes = Vec::new();
    pixmap.write_to(&mut bytes, ImageFormat::PNG)?;
    Ok(bytes)
}

/// Extract raster image occurrences as PNG bytes, filtered by displayed side lengths in points.
#[pyfunction(signature = (filename, min_long_side=0, min_short_side=0))]
fn get_images(
    py: Python<'_>,
    filename: String,
    min_long_side: u32,
    min_short_side: u32,
) -> PyResult<Vec<Py<PyBytes>>> {
    // Running page content also resolves fonts while collecting images.
    ensure_fontconfig_cache_dir();

    let document = mupdf::pdf::document::PdfDocument::open(&filename)
        .map_err(|err| PyIOError::new_err(err.to_string()))?;
    let mut images = Vec::new();
    for page in document.pages().map_err(to_pyerr)? {
        let collected = Rc::new(RefCell::new(ImageCollector::default()));
        let device = Device::from_native(collected.clone()).map_err(to_pyerr)?;
        page.map_err(to_pyerr)?
            .run(&device, &Matrix::IDENTITY)
            .map_err(to_pyerr)?;
        drop(device);

        for (image, placement) in &collected.borrow().0 {
            // The placement maps a unit square to the page; edge lengths handle rotation/skew.
            let width = f64::from(placement.a).hypot(f64::from(placement.b));
            let height = f64::from(placement.c).hypot(f64::from(placement.d));
            // Filter before decoding to avoid allocating pixels for rejected images.
            if width.max(height) < f64::from(min_long_side)
                || width.min(height) < f64::from(min_short_side)
            {
                continue;
            }

            let bytes = image_to_png(image).map_err(to_pyerr)?;
            images.push(PyBytes::new(py, &bytes).unbind());
        }
    }
    Ok(images)
}

/// A Python module implemented in Rust.
#[pymodule]
fn fast_pdf_extract(m: &Bound<'_, PyModule>) -> PyResult<()> {
    m.add_function(wrap_pyfunction!(get_page_count, m)?)?;
    m.add_function(wrap_pyfunction!(get_pages, m)?)?;
    m.add_function(wrap_pyfunction!(get_images, m)?)?;
    Ok(())
}
