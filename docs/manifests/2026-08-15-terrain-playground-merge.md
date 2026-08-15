# Terrain playground — land it on main

**Status:** ACTIVE 2026-08-15 — implemented on a branch, unverified, unmerged
**Branch:** `worktree-terrain-playground`

Observable: from a clean `main` checkout, opening `tools/map_viewer.html`
generates a landscape in the browser — no `map.json`, no shell run — and a
map saved by name comes back after a reload.

## State on arrival (surveyed 2026-08-15)

Found during the workflow conversion, not planned into it. The branch is
**five commits ahead of `main` and zero behind**, and its worktree is live at
`.claude/worktrees/terrain-playground`:

| commit | what it added |
| --- | --- |
| `11d538c` | parity canary test pinning `Terrain::generate` |
| `746bf5c` | in-browser terrain generation, parity-pinned to Rust |
| `dc64a1d` | playground controls — seed, random, size, shape |
| `5a312e7` | save and reselect named maps via localStorage |
| `12575ac` | file-load error reporting; playground recorded in CLAUDE.md |

Every task in `docs/superpowers/plans/2026-08-05-terrain-playground.md` has
a commit against it, so the work looks complete. What is *not* established:
each of that plan's five "verify in a real browser" steps is an unticked
checkbox, and no pack gate was run — the branch predates
`scripts/verify.sh` existing. So this pack assumes nothing about behavior
and re-establishes it.

## Decisions

- **Verify before merging, not after.** The parity invariant (JS elevations
  integer-for-integer identical to Rust at the shipped constants) is the one
  claim a type-checker cannot make, and the canary exists precisely because
  drift is silent. It gets exercised in a real browser console.
- **The branch's CLAUDE.md edit is the merge's known conflict.** `12575ac`
  rewrote the terrain bullet on the branch while the conversion rewrites
  neighbouring sections on `main`. Resolve toward the branch for the terrain
  bullet and toward `main` for everything else.

## Items

- [ ] **1. Verify the playground in a real browser.** Open the branch's
  `tools/map_viewer.html` and exercise each claim: startup shows a generated
  map, Random and Generate produce different landscapes, the size and octave
  inputs change the result and clamp at their limits, Save / reselect /
  Delete round-trip across a reload, and a malformed JSON drop reports
  rather than breaks. ⚠ The parity canary is the one claim no type-checker
  can make, and drift in it is silent.
  Done: every claim seen to work, and the console shows the parity
  self-check passing with no errors beside it.
- [ ] **2. Pack gate on the branch.**
  Done: `./scripts/verify.sh` prints `VERIFY OK`, with the test count quoted
  and the parity canary among the tests that ran.
- [ ] **3. Merge to `main` and retire the worktree.** Resolve the CLAUDE.md
  conflict as decided above.
  Done: `git log main --oneline` lists all five commits, `./scripts/verify.sh`
  is green on `main`, and `git worktree list` no longer shows the playground
  worktree.
- [ ] **4. Fold the 🚧 line into a real inventory entry.**
  Done: `docs/INVENTORY.md` describes browser generation and saved maps in
  the present tense and carries no 🚧 pointer at this manifest.

## Not in this pack unless you say so

- Any in-sim consumer of terrain — agents still have no position and
  buildings no footprint. The viewer stays a viewer.
- Terrain features from `docs/ideas.md` (ore fields, roads, water). Those
  graduate through brainstorming, not through a merge.
- The stale `main-1` worktree, retired separately during the conversion.

## Ledger

- **2026-08-15** — pack opened during the workflow conversion, which is how
  the unmerged branch surfaced. Nothing about the branch's behavior is taken
  on trust: its own plan's browser checks are unticked and it has never met
  a pack gate.
