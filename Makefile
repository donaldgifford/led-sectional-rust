include .env
export

.PHONY: publish
publish:
	cargo publish -p led-sectional-core --token "$(CRATES_IO_TOKEN)"
