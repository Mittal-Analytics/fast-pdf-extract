# fast-pdf-extract

Rust backed PDF text extraction library for Python.

## Features

- Detect and remove headers and footers
- Clean bilingual PDFs
- Mark headings in bold (basic Markdown)
- High accuracy
- Performance

## Extract images

```python
from pathlib import Path
from fast_pdf_extract import get_images

images = get_images("document.pdf", min_long_side=500, min_short_side=300)
for index, png in enumerate(images):
    Path(f"image-{index}.png").write_bytes(png)
```

`get_images(filename, min_long_side=0, min_short_side=0) -> list[bytes]`
returns raster image occurrences from PDF pages as PNG bytes in page order.
Repeated occurrences are retained, including images reused across pages. Inline
images are included; unused image resources and vector artwork are not.

Both thresholds are inclusive, non-negative integer **page points** (1/72 inch,
equivalent to pixels at 72 DPI), not native image pixels or viewer-dependent screen
pixels. Each occurrence is filtered by its displayed side lengths on the page:
`max(width, height) >= min_long_side` and `min(width, height) >= min_short_side`.
Scaling, rotation, and skew are accounted for using the transformed image edges,
not an axis-aligned bounding box. Clipping does not reduce these side lengths.

The defaults disable size filtering. Returned PNGs retain their native pixel
dimensions; filtering does not resize them. An image with many native pixels can
therefore be excluded when displayed small on the page, while a low-resolution
image displayed large can pass.

Images are exported in RGB with transparency (including soft masks), rather than
in their original compressed format. Stencil masks are exported in black with
transparency. Page placement affects filtering, but exported images are not rotated
or clipped. This is image extraction, not page rendering.

Returns an empty list when no images qualify. File-opening errors raise `OSError`;
extraction errors propagated by MuPDF raise `ValueError`. MuPDF can recover
some malformed PDFs or damaged images without raising an error, so successful
extraction does not guarantee that the source was intact.

## Development

```
uv sync --only-dev

# run tests (it rebuilds automatically)
uv run python -m unittest

# updating dependencies
cargo update
uv lock --upgrade
```

## Publishing a new version

1. Check the latest published version.

```bash
python - <<'PY'
import json
import urllib.request

with urllib.request.urlopen("https://pypi.org/pypi/fast-pdf-extract/json") as response:
    data = json.load(response)

print(data["info"]["version"])
PY
```

2. Bump the version in `Cargo.toml`.

```toml
[package]
version = "0.6.1"
```

3. Refresh lockfiles and run checks.

```bash
cargo check
uv lock
just test
```

4. Commit and push the release changes.

```bash
git add Cargo.toml Cargo.lock uv.lock
# Add any source changes that should be part of the release.
git commit -m "Bump version to <version>"
git push
```

5. Create and push a version tag.

```bash
git tag v<version>
git push origin v<version>
```

The `Release` GitHub workflow builds the Linux Python 3.13+ ABI3 wheel and sdist in
`ghcr.io/astral-sh/uv:python3.13-bookworm` and publishes them to PyPI using
Trusted Publishing. Push a `v<version>` tag to publish a release.

6. Verify PyPI shows the new version.

```bash
python - <<'PY'
import json
import urllib.request

with urllib.request.urlopen("https://pypi.org/pypi/fast-pdf-extract/json") as response:
    data = json.load(response)

print(data["info"]["version"])
PY
```

To build the same Linux wheel and sdist locally, run:

```bash
docker run --rm --platform linux/amd64 \
  -v "$PWD:/io" \
  -w /io \
  ghcr.io/astral-sh/uv:python3.13-bookworm \
  bash -lc '
    apt-get update &&
    apt-get install -y --no-install-recommends \
      build-essential ca-certificates clang curl \
      libfontconfig1-dev llvm patchelf pkg-config &&
    curl --proto "=https" --tlsv1.2 -sSf https://sh.rustup.rs \
      | sh -s -- -y --profile minimal &&
    . "$HOME/.cargo/env" &&
    uvx --from "maturin>=1.15,<2.0" maturin build \
      --release \
      --interpreter python3.13 \
      --compatibility manylinux_2_36 \
      --auditwheel repair \
      --out dist \
      --target-dir /tmp/fast-pdf-extract-target &&
    uvx --from "maturin>=1.15,<2.0" maturin sdist --out dist
  '
```

### Troubleshooting

If `cargo build` complains of missing python version.

```
cargo clean
cargo build
```
