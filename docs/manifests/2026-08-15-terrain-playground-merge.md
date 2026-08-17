# Terrain playground — land it on main

**Status:** COMPLETE 2026-08-15 — merged to `main`, gate green, inventory folded
**Branch:** `worktree-terrain-playground` → merged into `main` at `0d926ab`

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

- [x] **1. Verify the playground in a real browser.** Open
  `.claude/worktrees/terrain-playground/tools/map_viewer.html` — the live
  worktree for this branch — and exercise each claim: startup shows a generated
  map, Random and Generate produce different landscapes, the size and octave
  inputs change the result and clamp at their limits, Save / reselect /
  Delete round-trip across a reload, and a malformed JSON drop reports
  rather than breaks. ⚠ The parity canary is the one claim no type-checker
  can make, and drift in it is silent.
  Done: every claim seen to work, and the console shows the parity
  self-check passing with no errors beside it.
- [x] **2. Pack gate on the branch.**
  Done: `./scripts/verify.sh` prints `VERIFY OK`, with the test count quoted
  and the parity canary among the tests that ran.
- [x] **3. Merge to `main` and retire the worktree.** Resolve the CLAUDE.md
  conflict as decided above, then `git worktree remove
  .claude/worktrees/terrain-playground`. The worktree is gitignored, so it
  leaves no trace once removed — check it out again from the branch if the
  merge has to be redone.
  Done: `git log main --oneline` lists all five commits, `./scripts/verify.sh`
  is green on `main`, and `git worktree list` no longer shows the playground
  worktree.
- [x] **4. Fold the 🚧 line into a real inventory entry.**
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
- **2026-08-15** — item 1 under way. Served the worktree's viewer over
  `http://localhost:7431` (`.claude/launch.json`, config `map-viewer`) —
  opening it as a `file://` snapshot runs no scripts and gives no
  localStorage, so it proves nothing. Verified so far, in the page:
  - **Parity canary passes in the browser**: console logs
    `terrain parity canary ok` on load.
  - **Parity holds beyond the two pinned vertices.** Loading the Rust
    `map` export of seed 20260728 and then generating the same map in the
    browser produced *pixel-identical* canvases (canvas fingerprint
    `3d2592d4` both times). The export's first elevation is 235, the
    canary's pinned (0, 0).
  - **Random** produced seed 9377536411959497377 with a changed canvas and
    matching status line; **Generate** redrew on parameter change.
  - **Clamps write back**: vx 9999 → 512, octave period 0 → 1, both
    reflected in the inputs and in the redrawn map.
  - **Save / reselect survive a reload**: the record is parameters only,
    seed kept as a decimal string (`12345678901234567890` round-tripped
    exactly, past Number's safe range), and reselecting after F5 restored
    every input and regenerated.
  - **Bad file loads report**: `could not parse broken.json: …` for
    malformed JSON, `notamap.json is not a map.json export` for valid JSON
    of the wrong shape.
  - **Delete**: the handler is correct — invoking it empties the store and
    sets `deleted "alpha"` — but this was *not* confirmed through a pointer
    click, see below.
- **2026-08-15** — **item 1 blocked on real pointer input.** The Browser
  pane is not displayed, so the page composites no frames: the canvas came
  up 0×0 until a `resize` event was dispatched, screenshots time out, and
  synthetic clicks stopped reaching the page entirely (an instrumented
  capture listener on Generate — a button that had worked minutes earlier —
  recorded no `mousedown`, `mouseup` or `click`). So drag-to-rotate,
  wheel-to-zoom, and clicking as a human remain unverified, and nobody has
  actually *looked* at the render. Not merging on measurements alone.
- **2026-08-15** — **item 1 done.** Pane displayed, page compositing, every
  remaining claim exercised through real pointer input and seen on screen:
  - The playground opens on a hillshaded isometric landscape at the default
    seed — looked at, not inferred.
  - **Drag rotates** and **wheel zooms**: same landscape from a new angle,
    then filling more of the view.
  - **Random** clicked as a human → seed 5528872770560469304, a different
    landscape, status line following.
  - **Save** typed and clicked → `saved "gamma"`, the record holding the
    seed as a string; it survived a reload still listed in the dropdown.
  - **Delete** clicked → `deleted "beta"`, store back to `{}`, dropdown back
    to its placeholder. This is the one that had looked broken while input
    was not landing; with a real click it works.
  - Nuance worth keeping: **picking an option from the saved-maps dropdown**
    was driven through the `change` event its handler listens on, not by
    clicking an option in the native popup — that popup is an OS widget the
    automation cannot address. Restoring parameters and regenerating from a
    saved record is proven; the popup itself is not.
- **2026-08-15** — item 2 done, with a **deviation**: the branch predates
  `scripts/`, so there is no `./scripts/verify.sh` on it to run. Ran the
  identical gate by hand in the worktree instead — `cargo fmt --check`
  clean, `cargo clippy --all-targets -- -D warnings` exit 0 with no
  warnings, `cargo test` **99 passed, 0 failed**, including
  `terrain::tests::generate_matches_viewer_canary`. The merge brings the
  scripts onto the branch's history, so item 3's gate run is the real one.
- **2026-08-15** — items 3 and 4 done; **pack closed.** The CLAUDE.md
  conflict the Decisions section braced for never materialised: git
  auto-merged the branch's terrain bullet alongside the conversion's gate
  commands and inventory pointer, so no hand resolution was needed. Pack
  gate on the merged tree: `./scripts/verify.sh` → `VERIFY OK`, **99 tests
  passed**. All five commits are on `main` under merge `0d926ab`, and
  `git worktree list` shows only the main checkout. The inventory's 🚧 line
  is now a real entry describing in-page generation, saved maps and
  map.json loading, and the section is retitled "Terrain and the map
  playground" — the viewer is not only a viewer any more.
- **2026-08-15** — housekeeping alongside the fold: `.claude/launch.json`'s
  `map-viewer` config now serves `tools/` from the repo root rather than the
  retired worktree, and CLAUDE.md's in-flight note became a landed note.
