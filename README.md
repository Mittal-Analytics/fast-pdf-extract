# fast-pdf-extract

Rust backed PDF text extraction library for Python.

## Features

- Detect and remove headers and footers
- Clean bilingual PDFs
- Mark headings in bold (basic markdown)
- High accuracy
- Performance


## Development

```
uv sync --only-dev

# run tests
python -m unittest

# publishing
maturin build --release
maturin publish
```

