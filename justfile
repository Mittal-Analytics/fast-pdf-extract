set fallback

[positional-arguments]
@test *args='':
	uv run python -m unittest $@

