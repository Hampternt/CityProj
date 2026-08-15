#!/usr/bin/env bash
# The fast ITEM gate for CityProj — run after every item, before its commit.
#
# The workflow gates in two tiers:
#   item gate — this script: fmt, clippy, build. Seconds.
#   pack gate — ./scripts/verify.sh: this plus the full test suite. Run before
#               a pack's review and walkthrough; nothing merges without it.
#
# What this script can NOT vouch for: logic. Rust's type checker says nothing
# about whether money still balances or a price moved the right way. If the
# item touches the economy (money, wages, market, goods, sim phases), also run
# that area's tests, e.g.  cargo test money::  /  cargo test market::
#
# clippy runs with -D warnings: the tree was clean when this gate was written
# (2026-08-15) and staying clean is cheaper than triaging a warning backlog.
#
# Exit 0 = fmt, clippy and build clean. On failure every check still runs, so
# one invocation shows every problem.

set -uo pipefail

cd "$(dirname "${BASH_SOURCE[0]}")/.." || exit 1

failed=()

run() {
  local name="$1"
  shift
  printf '\n=== %s\n' "$name"
  "$@" || failed+=("$name")
}

run "cargo fmt --check" cargo fmt --check
run "cargo clippy" cargo clippy --all-targets -- -D warnings
run "cargo build" cargo build

printf '\n'
if (( ${#failed[@]} == 0 )); then
  printf 'CHECK OK — fmt, clippy, build clean. (Logic touched? Run its tests too.)\n'
  exit 0
fi

printf 'CHECK FAILED: %s\n' "${failed[*]}"
exit 1
