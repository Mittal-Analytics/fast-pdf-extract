use std::collections::HashMap;

use mupdf::page::StextPage;
use pyo3::{
    exceptions::{PyIOError, PyValueError},
    prelude::*,
};

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

    bad_words_count > 4 && bad_words_count as f64 > original_count as f64 * 0.1
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
        if after_page.len() > 0 {
            after_pages.push(after_page);
        };
    }

    if (after_len as f64) < (0.9 * before_len as f64) {
        after_pages
    } else {
        pages
    }
}

#[pyfunction]
fn get_pages(filename: String) -> PyResult<Vec<String>> {
    let document = mupdf::pdf::document::PdfDocument::open(&filename)
        .map_err(|err| PyIOError::new_err(err.to_string()))?;
    let mut pages = document
        .pages()
        .map_err(to_pyerr)?
        .filter_map(|page| {
            let stext_json = page.ok()?.stext_page_as_json_from_page(1.0).ok()?;
            let stext_page: StextPage = serde_json::from_str(stext_json.as_str()).ok()?;
            Some(get_styled_paragraphs(stext_page))
        })
        .collect::<Pages>();

    remove_headers_footers(&mut pages);
    let pages = remove_non_english(pages);
    Ok(pages
        .iter()
        .map(|paragraphs| paragraphs.join("\n\n"))
        .collect())
}

/// A Python module implemented in Rust.
#[pymodule]
fn fast_pdf_extract(m: &Bound<'_, PyModule>) -> PyResult<()> {
    m.add_function(wrap_pyfunction!(get_pages, m)?)?;
    Ok(())
}
