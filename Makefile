.PHONY: check
check:
	cargo check

.PHONY: checkall
checkall:
	cargo check --all-targets --all-features

.PHONY: build
build:
	cargo build

.PHONY: buildall
buildall:
	cargo build --all-targets --all-features

.PHONY: test
test: TEST = ''
test:
	cargo test --lib --verbose -- $(TEST)

integrationtests: FILE = *
integrationtests: TEST = ''
integrationtests:
	cargo test --test '$(FILE)' -- $(TEST)

.PHONY: testall
testall: test integrationtests

.PHONY: fmt
fmt:
	cargo fmt -- --check

.PHONY: clippy
clippy:
	cargo clippy -- -D warnings

.PHONY: udeps
udeps:
	cargo +nightly udeps

# Quick tests to run before creating a PR.
.PHONY: pr
pr: fmt buildall testall clippy
