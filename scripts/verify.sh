#!/usr/bin/env bash
# The PACK gate for CityProj — the full verification suite.
#
# Every pack's acceptance line is `./scripts/verify.sh`: run it and quote its
# output. "It compiles" is an item-gate claim, not this one. Nothing merges
# without a green run here.
#
#   1. ./scripts/check.sh — fmt, clippy (-D warnings), build
#   2. cargo test         — the whole suite, including the conservation audit
#                           tests that guard the §8 money invariants
#
# Exit 0 = everything passed. On failure every check still runs, so one
# invocation shows every problem.

set -uo pipefail

cd "$(dirname "${BASH_SOURCE[0]}")/.." || exit 1

failed=()

printf '=== ./scripts/check.sh\n'
./scripts/check.sh || failed+=("check.sh")

printf '\n=== cargo test\n'
cargo test || failed+=("cargo test")

printf '\n'
if (( ${#failed[@]} == 0 )); then
  printf 'VERIFY OK — fmt, clippy, build, tests all clean.\n'
  exit 0
fi

printf 'VERIFY FAILED: %s\n' "${failed[*]}"
exit 1
