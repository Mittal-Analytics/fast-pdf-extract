# Release process

- Releases are published from GitHub Actions using PyPI Trusted Publishing.
- Bump `Cargo.toml`, refresh `Cargo.lock`/`uv.lock`, commit, push `main`, then tag the commit as `v<version>` and push the tag.
- The `Release` workflow builds the Linux Python 3.13+ ABI3 wheel plus sdist and publishes them to PyPI when a `v*` tag is pushed.
