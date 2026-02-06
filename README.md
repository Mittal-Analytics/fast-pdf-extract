# fast-pdf-extract

Rust backed PDF text extraction library for Python.

## Features

- Detect and remove headers and footers
- Clean bilingual PDFs
- Mark headings in bold (basic Markdown)
- High accuracy
- Performance


## Development

```
uv sync --only-dev

# run tests (it rebuild automatically)
uv run python -m unittest

# publishing
maturin build --release
maturin publish
```

### Troubleshooting

If `cargo build` complains of missing python version.

```
cargo clean
cargo build
```
