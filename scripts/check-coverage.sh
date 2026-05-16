#!/usr/bin/env sh
set -eu

minimum="${REWEAVE_COVERAGE_MIN:-99.5}"

exec cargo llvm-cov --workspace --summary-only --fail-under-lines "$minimum" "$@"
