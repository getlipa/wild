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

# Check that we stick to `mod tests {` style.
.PHONY: check-mod-test
check-mod-test:
	! grep --recursive --include="*.rs" "mod test " *

.PHONY: check-version
check-version:
	@echo "Check Cargo.toml version against git version"
	@toml_version=`sed -n -e 's/^version = "\(.*\)"/\1/p' Cargo.toml`; \
	git_version=`git describe --tags $$(git rev-list --tags --max-count=1) | cut -c2-`; \
	[ "$$toml_version" = "$$git_version" ] || \
		(echo "Cargo.toml version ($$toml_version) does not match git version ($$git_version)!" &&  \
		exit 1)

# Quick tests to run before creating a PR.
.PHONY: pr
pr: fmt buildall testall clippy check-mod-test check-version
