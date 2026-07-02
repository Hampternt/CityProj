# <Feature> — Design Spec

<!--
Fill top-to-bottom. Design section = decisions (the veto point). Contracts =
load-bearing I/O. Then STOP at the gate and get approval before the plan.
Keep it short; scale each section to its actual complexity.
-->

## Design (decisions — approve before reading on)

<One paragraph: what this builds and why. The direction someone could veto in
10 seconds. For a small feature this is a few sentences.>

- **Fits into:** <which §7 modules / files this touches — @src/... refs>
- **Invariants in play:** <which of CLAUDE.md §8 this must honor, e.g. §8.2 single chokepoint>
- **Out of scope (YAGNI):** <what we are deliberately NOT doing in v1>

## Contracts (the hard reference the plan is written from)

<!--
CONTRACT-WORTHY = load-bearing. Write a contract for a unit when it:
  - crosses a module/unit boundary (something else calls it), OR
  - enforces an invariant, OR
  - is the feature's entry point, OR
  - could behave more than one way (the Given/Then forces the choice).

NOT contract-worthy — leave to the plan/implementation:
  - single-caller internal helpers
  - behavior obvious from the signature
  - mechanical decomposition (splitting a loop into a helper)

The plan WILL discover sub-functions these need. That is expected and correct —
the plan owns their signatures. If a discovered helper turns out to ALSO be
load-bearing (shared across units / touches an invariant), lift it back up here
and re-approve.
-->

### Unit: <name>  (<file.rs>)

Signature: `fn <name>(<args with types>) -> <Return / Result<_, Error>>`
Given:  <precondition / input state>
Then:   <postcondition / what changes — name the invariant it preserves>
Error:  <bad input> -> `Err(<Variant>)`   // <which invariant this defends>
Refs:   @src/<file>.rs, §8.<n> <invariant name>

<!-- repeat per load-bearing unit -->

### Mechanical / obvious (plan owns these — no contract needed)

<Name the units that are intentionally left unspecified so the plan-writer
does NOT invent interpretations. e.g. "render helpers, ID counters — trivial.">

--- APPROVAL GATE — do not write the plan or any code above this line without sign-off ---

## Open questions

<Anything genuinely undecided. Empty = nothing ambiguous survived. Resolve
these before the gate, don't defer them into the plan.>
