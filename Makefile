.SUFFIXES:

SHELL := /bin/bash

.PHONY: all
all: autobind

# ----------------------------------------------------------------------

.PHONY: autobind
autobind:
	@cd autobind && cargo build --verbose

# ----------------------------------------------------------------------

.PHONY: examples
examples: build/test
	@echo ""
	@echo "Running example:"
	@echo "================"
	@./build/test

build/test: examples/test.rs autobind
	RUST_LOG=autobind=debug rustc -g -L autobind/target -o $@ $<

# ----------------------------------------------------------------------

.PHONY: test
test:
	@cd autobind && cargo test

# ----------------------------------------------------------------------

COMMON_COMPILETEST_FLAGS := \
	--compile-lib-path=/usr/lib 						\
	--run-lib-path=/usr/lib 							\
	--rustc-path=`which rustc` 							\
	--build-base=./build/ 								\
	--target-rustcflags="-L `pwd`/autobind/target"		\
	--aux-base=./tests/auxiliary/ 						\
	--android-cross-path=/tmp							\
	--stage-id=stage2									\
	--target=x86_64-unknown-linux-gnu

.PHONY: compiletest
compiletest: compiletest-compile-fail compiletest-run-pass

.PHONY: compiletest-binary
compiletest-binary:
	@command -v compiletest &>/dev/null || { echo "ERROR: compiletest binary not found"; exit 1; }
	@#compiletest --help &>/dev/null || { echo "ERROR: running compiletest failed - is your LD_LIBRARY_PATH correct?"; exit 1; }

.PHONY: compiletest-compile-fail
compiletest-compile-fail: compiletest-binary
	compiletest 											\
		--mode=compile-fail 								\
		--src-base=./tests/compile-fail/					\
		$(COMMON_COMPILETEST_FLAGS)

.PHONY: compiletest-run-pass
compiletest-run-pass: compiletest-binary
	compiletest 											\
		--mode=run-pass 									\
		--src-base=./tests/run-pass/						\
		$(COMMON_COMPILETEST_FLAGS)

# ----------------------------------------------------------------------

.PHONY: env
env:
	@echo
