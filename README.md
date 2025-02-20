# fast-pdf-extract

A Rust backed PDF text extraction library for Python.

## Features

- Detect and remove headers and footers
- Clean bilingual PDFs
- Mark headings in bold (basic markdown)
- High accuracy
- Peformance


## Development

```
uv sync --only-dev

# run tests
python -m unittest

# publishing
maturin build --release
maturin publish
```

