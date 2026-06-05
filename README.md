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

4. Build the release artifacts.

```bash
rm -rf target/wheels dist
uv run maturin build --release
```

5. Publish to PyPI.

```bash
# MATURIN_PYPI_TOKEN must be set in the environment.
uv run maturin publish --skip-existing
```

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

7. Commit the version bump.

```bash
git add Cargo.toml Cargo.lock
git commit -m "Bump version to <version>"
```

### Troubleshooting

If `cargo build` complains of missing python version.

```
cargo clean
cargo build
```
