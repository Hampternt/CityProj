# Terrain Playground Implementation Plan

> **For agentic workers:** REQUIRED SUB-SKILL: Use superpowers:subagent-driven-development (recommended) or superpowers:executing-plans to implement this plan task-by-task. Steps use checkbox (`- [ ]`) syntax for tracking.

**Goal:** Grow `tools/map_viewer.html` into a terrain playground — generate random maps in the browser, tweak parameters, save/reselect named maps — with a JS value-noise port kept bit-identical to Rust at the shipped constants.

**Architecture:** All UI and generation lives in the single self-contained `tools/map_viewer.html` (no build step, no server, no external requests). The JS generator mirrors `Terrain::generate` in `src/terrain.rs` operation-for-operation, using BigInt for the 64-bit hash. A pinned-value canary on both sides (a Rust test and a JS startup self-check) catches drift. The existing render path (`rotated`/`project`/`draw`, hillshade, drag/zoom) and the `map.json` file input are untouched except where named below.

**Tech Stack:** Rust (edition 2024, tests only — no behavior change), vanilla JS + Canvas in one HTML file, `localStorage` for saved maps.

**Spec:** `docs/superpowers/specs/2026-08-05-terrain-playground-design.md`. Its Contracts section is the source of truth for names and behavior; this plan copies them verbatim.

## Global Constraints

- `tools/map_viewer.html` stays a single self-contained file: no external scripts, styles, fonts, or network requests.
- No new Rust dependencies; no changes to `Terrain::generate`'s signature, constants, or behavior.
- **Parity invariant (spec, `generateTerrain`):** at default parameters — octaves (16, 1.0), (8, 0.5), (4, 0.25), max elevation 400 — JS elevations are integer-for-integer identical to Rust `Terrain::generate(seed, vx, vy, cell)` for every seed.
- Rounding in JS is `Math.round(v)` (exact nearest, ties toward +∞ — identical to Rust's half-away-from-zero for the non-negative values here). Never `Math.floor(v + 0.5)`.
- The in-memory map shape the renderer consumes stays `{cell_size, vertices_x, vertices_y, elevations}` (snake_case, row-major) — the `Terrain::to_json` contract.
- Canary pins (spec): seed 20260728, 64×64, cell 50, default shape → vertex (0, 0) = **235**, vertex (33, 17) (world (1650, 850)) = **183**.
- localStorage key: `"cityproj.saved_maps"`; records are parameters only, never elevation arrays; `seed` stored as a decimal string (u64 exceeds Number's exact range).
- Input clamps (spec): dims 2–512, cell size ≥ 1, period ≥ 1, amplitude > 0, max elevation ≥ 1; seed parsed as u64, invalid → 0.
- UI/browser verification is real: open the file in a browser and check behavior + console, don't just re-read the code.
- Run `cargo fmt` before each Rust commit.

---

### Task 1: Rust parity-canary test

**Files:**
- Modify: `src/terrain.rs` (tests module only — add one test after `generate_has_visible_relief_at_shipped_size`, around line 443)

**Interfaces:**
- Consumes: existing `Terrain::generate`, `Terrain::elevation_at`.
- Produces: the two pinned canary values (235, 183) that Task 2's JS self-check asserts against. No code interface.

- [ ] **Step 1: Write the test**

Add to the `tests` module in `src/terrain.rs`, directly after `generate_has_visible_relief_at_shipped_size`:

```rust
    #[test]
    fn generate_matches_viewer_canary() {
        // Pinned parity values shared with tools/map_viewer.html's startup
        // canary (spec, "parity canary" unit): if either side drifts from
        // the other, one of the two checks fails.
        let t = Terrain::generate(20260728, 64, 64, 50);
        // Vertex (0, 0) and vertex (33, 17) — world (1650, 850) at cell
        // size 50; elevation_at returns stored samples exactly at vertices.
        assert_eq!(t.elevation_at(0, 0), Ok(235));
        assert_eq!(t.elevation_at(1650, 850), Ok(183));
    }
```

- [ ] **Step 2: Run the test — it must pass immediately**

Run: `cargo test generate_matches_viewer_canary`
Expected: `test result: ok. 1 passed`.

This test pins *existing* behavior, so it passes on first run. **If it fails, STOP** — the spec's pinned values are wrong; do not "fix" `generate`. Report the actual values instead.

- [ ] **Step 3: Full test suite + lint**

Run: `cargo fmt && cargo clippy && cargo test`
Expected: clippy clean, all tests pass (98 existing + 1 new).

- [ ] **Step 4: Commit**

```bash
git add src/terrain.rs
git commit -m "test: pin generate parity canary for the viewer's JS port"
```

---

### Task 2: JS generator port + startup canary

**Files:**
- Modify: `tools/map_viewer.html` (script block; also the input-contract comment at the top of the script)

**Interfaces:**
- Consumes: existing globals `map`, `fitZoom()`, `draw()`, `resize()` in the same script.
- Produces: `generateTerrain(params)` and `DEFAULT_PARAMS`, used by Tasks 3–4. `params` is `{seed: BigInt, verticesX, verticesY, cellSize, maxElevation, octaves: [[period, amplitude], …]}`; returns `{cell_size, vertices_x, vertices_y, elevations}` (the render shape). Also `parityCanary()` (returns the default-params map).

- [ ] **Step 1: Add the generator**

Insert after the `normalize` function (line ~34), before the file-input listener:

```js
// ── Terrain generation — JS port of src/terrain.rs generate ──────────
// Parity invariant (spec): at the default parameters below, elevations
// are integer-for-integer identical to Rust Terrain::generate for every
// seed. The hash is bit-exact via BigInt; the float blending mirrors
// Rust's operation order in plain f64 doubles. src/terrain.rs is the
// source of truth — change these together.
const U64 = (1n << 64n) - 1n;

const DEFAULT_PARAMS = {
  seed: 20260728n,
  verticesX: 64,
  verticesY: 64,
  cellSize: 50,
  maxElevation: 400,
  octaves: [[16, 1.0], [8, 0.5], [4, 0.25]],  // Rust NOISE_OCTAVES
};

// Splitmix64-style avalanche, same constants as Rust lattice_value.
function latticeValue(seedBig, octaveBig, ix, iy) {
  let h = seedBig
    ^ (octaveBig * 0x9E3779B97F4A7C15n & U64)
    ^ (BigInt(ix) * 0xC2B2AE3D27D4EB4Fn & U64)
    ^ (BigInt(iy) * 0x165667B19E3779F9n & U64);
  h = (h ^ (h >> 30n)) * 0xBF58476D1CE4E5B9n & U64;
  h = (h ^ (h >> 27n)) * 0x94D049BB133111EBn & U64;
  h ^= h >> 31n;
  return Number(h >> 11n) / Number(1n << 53n);
}

// The classic cubic fade t²(3 − 2t), as in Rust smoothstep.
function fade(t) {
  return t * t * (3.0 - 2.0 * t);
}

// One octave of value noise at vertex (vx, vy) — mirrors Rust value_noise.
function valueNoise(seedBig, octave, vx, vy, period) {
  const ix = Math.floor(vx / period);
  const iy = Math.floor(vy / period);
  const sx = fade((vx % period) / period);
  const sy = fade((vy % period) / period);
  const o = BigInt(octave);
  const v00 = latticeValue(seedBig, o, ix, iy);
  const v10 = latticeValue(seedBig, o, ix + 1, iy);
  const v01 = latticeValue(seedBig, o, ix, iy + 1);
  const v11 = latticeValue(seedBig, o, ix + 1, iy + 1);
  const south = v00 + sx * (v10 - v00);
  const north = v01 + sx * (v11 - v01);
  return south + sy * (north - south);
}

// Math.round, NOT Math.floor(v + 0.5): round is exact nearest (ties
// toward +∞ — same as Rust's half-away-from-zero for our v ≥ 0), while
// v + 0.5 can double-round.
function generateTerrain({ seed, verticesX, verticesY, cellSize, maxElevation, octaves }) {
  const seedBig = BigInt(seed) & U64;
  const totalAmplitude = octaves.reduce((sum, [, amplitude]) => sum + amplitude, 0);
  const elevations = [];
  for (let vy = 0; vy < verticesY; vy++) {
    for (let vx = 0; vx < verticesX; vx++) {
      let n = 0.0;
      for (let o = 0; o < octaves.length; o++) {
        const [period, amplitude] = octaves[o];
        n += amplitude * valueNoise(seedBig, o, vx, vy, period);
      }
      elevations.push(Math.round(n / totalAmplitude * maxElevation));
    }
  }
  return { cell_size: cellSize, vertices_x: verticesX, vertices_y: verticesY, elevations };
}

// Startup self-check against the values pinned by the Rust test
// generate_matches_viewer_canary (spec, "parity canary" unit).
function parityCanary() {
  const t = generateTerrain(DEFAULT_PARAMS);
  const got00 = t.elevations[0];
  const got3317 = t.elevations[17 * 64 + 33];
  if (got00 === 235 && got3317 === 183) {
    console.log("terrain parity canary ok");
  } else {
    console.warn(
      `terrain parity canary FAILED: (0,0)=${got00} want 235, `
      + `(33,17)=${got3317} want 183 — src/terrain.rs generate is the source of truth`);
  }
  return t;
}
```

- [ ] **Step 2: Show a generated map at startup**

At the bottom of the script, change:

```js
window.addEventListener("resize", resize);
resize();
```

to:

```js
map = parityCanary();  // playground opens showing the default-seed map
window.addEventListener("resize", resize);
resize();
```

(`resize()` already calls `fitZoom()` + `draw()` when `map` is set.)

- [ ] **Step 3: Update the input-contract comment**

Change the comment at the top of the script (lines ~20–22) from:

```js
// Input contract: {"unit_meters":0.1,"cell_size":C,"vertices_x":X,
// "vertices_y":Y,"elevations":[...]} row-major — produced by
// Terrain::to_json; this file and that serializer change together.
```

to:

```js
// Input contract: {"unit_meters":0.1,"cell_size":C,"vertices_x":X,
// "vertices_y":Y,"elevations":[...]} row-major — produced by
// Terrain::to_json; this file and that serializer change together.
// Maps can also be generated in-browser: generateTerrain below is a
// parity-pinned port of Terrain::generate (see the canary).
```

- [ ] **Step 4: Verify in a real browser**

Open `file:///home/hampter/projects/CityProj/tools/map_viewer.html` in a browser (claude-in-chrome if available, otherwise ask the user to open it).
Expected:
- Terrain renders immediately at load (no file needed) — rolling green/brown hills.
- Console shows `terrain parity canary ok` and no errors.
- Loading a shell-exported `map.json` for seed 20260728 via the file input shows the *same* terrain (parity end-to-end). Generate one with `cargo run` → `map` if an export is needed.

- [ ] **Step 5: Commit**

```bash
git add tools/map_viewer.html
git commit -m "feat: in-browser terrain generation, parity-pinned to Rust"
```

---

### Task 3: Control panel — seed, random, size, shape

**Files:**
- Modify: `tools/map_viewer.html` (the `#bar` div, CSS, and script wiring)

**Interfaces:**
- Consumes: `generateTerrain(params)`, `DEFAULT_PARAMS`, `fitZoom()`, `draw()` from Task 2.
- Produces: `readParams()` → the `generateTerrain` params object (clamped, from the inputs); `regenerate()`; `setStatus(text)`; input ids `seed`, `vx`, `vy`, `cell`, `maxelev`, `o0p`/`o0a`/`o1p`/`o1a`/`o2p`/`o2a`, `status`. Tasks 4–5 use all of these.

- [ ] **Step 1: Replace the bar markup**

Replace:

```html
<div id="bar">
  <input type="file" id="file" accept=".json,application/json">
  <span>load map.json · drag = rotate · wheel = zoom</span>
</div>
```

with:

```html
<div id="bar">
  <div class="row">
    <label>seed <input id="seed" type="text" value="20260728"></label>
    <button id="randomBtn">Random</button>
    <button id="genBtn">Generate</button>
  </div>
  <div class="row">
    <label>vx <input id="vx" type="number" min="2" max="512" value="64"></label>
    <label>vy <input id="vy" type="number" min="2" max="512" value="64"></label>
    <label>cell <input id="cell" type="number" min="1" value="50"></label>
    <label>max elev <input id="maxelev" type="number" min="1" value="400"></label>
  </div>
  <div class="row">
    octaves (period, amplitude):
    <input id="o0p" type="number" min="1" value="16"><input id="o0a" type="number" step="0.05" value="1">
    <input id="o1p" type="number" min="1" value="8"><input id="o1a" type="number" step="0.05" value="0.5">
    <input id="o2p" type="number" min="1" value="4"><input id="o2a" type="number" step="0.05" value="0.25">
  </div>
  <div class="row">
    <input type="file" id="file" accept=".json,application/json">
    <span id="status">drag = rotate · wheel = zoom</span>
  </div>
</div>
```

(Input defaults mirror `DEFAULT_PARAMS` — the startup map matches the panel.)

- [ ] **Step 2: Panel CSS**

In the `<style>` block, replace:

```css
  #bar { padding: 8px 12px; position: absolute; z-index: 1; }
```

with:

```css
  #bar { padding: 8px 12px; position: absolute; z-index: 1; margin: 6px;
         background: rgba(20, 20, 26, 0.85); border-radius: 6px; font-size: 13px; }
  #bar .row { margin-top: 4px; }
  #bar input[type=number] { width: 4.5em; }
  #bar #seed { width: 13em; }
```

- [ ] **Step 3: Wire the controls**

Add after the `parityCanary` function (order within the script doesn't matter beyond being before the startup lines):

```js
// ── Playground controls ──────────────────────────────────────────────
function setStatus(text) {
  document.getElementById("status").textContent = text;
}

// Clamp to [min, max] instead of erroring (spec, error handling), and
// write the clamped value back so the panel shows what was used.
function clampInt(el, min, max) {
  const raw = Math.floor(Number(el.value));
  const clamped = Math.min(Math.max(Number.isFinite(raw) ? raw : min, min), max);
  el.value = clamped;
  return clamped;
}

function clampFloat(el, min, max) {
  const raw = Number(el.value);
  const clamped = Math.min(Math.max(Number.isFinite(raw) ? raw : min, min), max);
  el.value = clamped;
  return clamped;
}

// u64 seed; anything unparseable or out of range → 0 (spec).
function parseSeed(text) {
  try {
    const s = BigInt(text.trim());
    return s >= 0n && s <= U64 ? s : 0n;
  } catch {
    return 0n;
  }
}

function readParams() {
  return {
    seed: parseSeed(document.getElementById("seed").value),
    verticesX: clampInt(document.getElementById("vx"), 2, 512),
    verticesY: clampInt(document.getElementById("vy"), 2, 512),
    cellSize: clampInt(document.getElementById("cell"), 1, 1000000),
    maxElevation: clampInt(document.getElementById("maxelev"), 1, 1000000),
    octaves: [0, 1, 2].map((i) => [
      clampInt(document.getElementById(`o${i}p`), 1, 512),
      clampFloat(document.getElementById(`o${i}a`), 0.01, 100),
    ]),
  };
}

function regenerate() {
  const params = readParams();
  document.getElementById("seed").value = params.seed.toString();
  map = generateTerrain(params);
  fitZoom();
  draw();
  setStatus(`generated seed ${params.seed}`);
}

document.getElementById("genBtn").addEventListener("click", regenerate);

document.getElementById("randomBtn").addEventListener("click", () => {
  const buf = new BigUint64Array(1);
  crypto.getRandomValues(buf);
  document.getElementById("seed").value = buf[0].toString();
  regenerate();
});
```

- [ ] **Step 4: Verify in a real browser**

Reload the page. Expected:
- **Random** produces a different map each click; the seed field shows the rolled seed.
- Typing seed `20260728` (defaults untouched) + **Generate** reproduces the startup map exactly.
- vx/vy `128` renders a larger map; `600` clamps back to 512 in the field; vx `1` clamps to 2.
- Raising max elev to `1200` visibly exaggerates relief; setting octave 0 period to `4` makes choppy hills; amplitude `0` clamps to 0.01.
- Seed field garbage (`abc`) generates seed 0 and the field shows `0`.
- Console stays error-free.

- [ ] **Step 5: Commit**

```bash
git add tools/map_viewer.html
git commit -m "feat: playground controls — seed, random, size, shape"
```

---

### Task 4: Saved maps — save, select, delete

**Files:**
- Modify: `tools/map_viewer.html` (one new bar row + script)

**Interfaces:**
- Consumes: `readParams()`, `regenerate()`, `setStatus()`, input ids from Task 3.
- Produces: localStorage key `"cityproj.saved_maps"` holding `{name: {seed: string, vertices_x, vertices_y, cell_size, max_elevation, octaves: [[period, amplitude], …]}}` (spec, "saved-map record" unit). Element ids `mapname`, `saveBtn`, `saved`, `deleteBtn`.

- [ ] **Step 1: Add the saved-maps row**

Insert into `#bar`, between the octaves row and the file row:

```html
  <div class="row">
    <input id="mapname" type="text" placeholder="map name">
    <button id="saveBtn">Save</button>
    <select id="saved"></select>
    <button id="deleteBtn">Delete</button>
  </div>
```

- [ ] **Step 2: Wire persistence**

Add after the Task 3 wiring code:

```js
// ── Saved maps: parameters only, never elevations — regeneration is
// deterministic (spec, "saved-map record" unit). Seed is a string
// because u64 exceeds Number's exact-integer range.
const STORE_KEY = "cityproj.saved_maps";

function loadStore() {
  // Missing/corrupt store → empty rather than crashing (spec).
  try {
    const parsed = JSON.parse(localStorage.getItem(STORE_KEY) || "{}");
    return parsed !== null && typeof parsed === "object" ? parsed : {};
  } catch {
    return {};
  }
}

function saveStore(store) {
  localStorage.setItem(STORE_KEY, JSON.stringify(store));
}

function refreshSavedList() {
  const select = document.getElementById("saved");
  select.innerHTML = "";
  const placeholder = document.createElement("option");
  placeholder.value = "";
  placeholder.textContent = "— saved maps —";
  select.appendChild(placeholder);
  for (const name of Object.keys(loadStore()).sort()) {
    const option = document.createElement("option");
    option.value = name;
    option.textContent = name;
    select.appendChild(option);
  }
}

document.getElementById("saveBtn").addEventListener("click", () => {
  const name = document.getElementById("mapname").value.trim();
  if (!name) {
    setStatus("name a map before saving");
    return;
  }
  const p = readParams();
  const store = loadStore();
  store[name] = {  // same name overwrites (spec)
    seed: p.seed.toString(),
    vertices_x: p.verticesX,
    vertices_y: p.verticesY,
    cell_size: p.cellSize,
    max_elevation: p.maxElevation,
    octaves: p.octaves,
  };
  saveStore(store);
  refreshSavedList();
  document.getElementById("saved").value = name;
  setStatus(`saved "${name}"`);
});

document.getElementById("saved").addEventListener("change", (event) => {
  const record = loadStore()[event.target.value];
  if (!record) return;
  document.getElementById("seed").value = record.seed;
  document.getElementById("vx").value = record.vertices_x;
  document.getElementById("vy").value = record.vertices_y;
  document.getElementById("cell").value = record.cell_size;
  document.getElementById("maxelev").value = record.max_elevation;
  record.octaves.forEach(([period, amplitude], i) => {
    document.getElementById(`o${i}p`).value = period;
    document.getElementById(`o${i}a`).value = amplitude;
  });
  regenerate();
});

document.getElementById("deleteBtn").addEventListener("click", () => {
  const name = document.getElementById("saved").value;
  if (!name) return;
  const store = loadStore();
  delete store[name];
  saveStore(store);
  refreshSavedList();
  setStatus(`deleted "${name}"`);
});

refreshSavedList();
```

- [ ] **Step 3: Verify in a real browser**

Reload. Expected:
- Roll a random map, name it `hills`, **Save** → appears selected in the dropdown; status `saved "hills"`.
- Change every parameter (different seed, 128×128, octaves), then reselect `hills` → all fields restore and the identical map re-renders.
- Save under `hills` again with different params → overwrites (still one `hills` entry).
- **Delete** removes it; reload page → deletions/saves persisted.
- DevTools → Application → localStorage: records contain parameters only (no `elevations`).
- Manually corrupt the key (`localStorage.setItem("cityproj.saved_maps", "{oops")` in the console), reload → page works, list is empty.

- [ ] **Step 4: Commit**

```bash
git add tools/map_viewer.html
git commit -m "feat: save and reselect named maps via localStorage"
```

---

### Task 5: File-load error reporting, docs, final sweep

**Files:**
- Modify: `tools/map_viewer.html` (file-input listener)
- Modify: `CLAUDE.md` (the `src/terrain.rs` bullet in "Current code state")

**Interfaces:**
- Consumes: `setStatus()`, `fitZoom()`, `draw()`.
- Produces: nothing new — hardening + docs.

- [ ] **Step 1: Report malformed file loads**

Replace the file-input listener:

```js
document.getElementById("file").addEventListener("change", (event) => {
  const file = event.target.files[0];
  if (!file) return;
  file.text().then((text) => {
    map = JSON.parse(text);
    fitZoom();
    draw();
  });
});
```

with:

```js
document.getElementById("file").addEventListener("change", (event) => {
  const file = event.target.files[0];
  if (!file) return;
  file.text().then((text) => {
    let parsed;
    try {
      parsed = JSON.parse(text);
    } catch (err) {
      setStatus(`could not parse ${file.name}: ${err.message}`);
      return;
    }
    if (!Number.isInteger(parsed.vertices_x) || !Number.isInteger(parsed.vertices_y)
        || !Array.isArray(parsed.elevations)) {
      setStatus(`${file.name} is not a map.json export`);
      return;
    }
    map = parsed;
    fitZoom();
    draw();
    setStatus(`loaded ${file.name}`);
  });
});
```

- [ ] **Step 2: Update CLAUDE.md's terrain bullet**

In the "Current code state" list, change the `src/terrain.rs` bullet's last sentence from:

```
  viewer. No in-sim consumer yet — the shell holds a display terrain and
  the `map` command exports `map.json` for `tools/map_viewer.html`
  (self-contained, open in a browser).
```

to:

```
  viewer. No in-sim consumer yet — the shell holds a display terrain and
  the `map` command exports `map.json` for `tools/map_viewer.html`
  (self-contained, open in a browser), which is also a terrain
  playground: in-browser generation parity-pinned to `generate` by the
  `generate_matches_viewer_canary` test, parameter knobs, and named maps
  saved to localStorage (parameters only; not wired into the sim).
```

- [ ] **Step 3: Verify in a real browser**

Reload. Expected:
- Loading a valid shell-exported `map.json` still works; status shows `loaded map.json`.
- Loading a non-JSON file (e.g. this plan's `.md`) shows `could not parse …` in the bar, no console exception, current map stays rendered.
- Loading a JSON file that isn't a map (e.g. `{"a":1}` saved to the scratchpad) shows `… is not a map.json export`.

- [ ] **Step 4: Full verification sweep**

Run: `cargo fmt && cargo clippy && cargo test`
Expected: clippy clean, all tests pass. Quote real output.

Browser: one end-to-end pass — startup map + canary `ok` in console, random/seed/size/shape generation, save → mutate → reselect restores, delete, file load, drag-rotate + wheel-zoom still smooth on a 128×128 map.

- [ ] **Step 5: Commit**

```bash
git add tools/map_viewer.html CLAUDE.md
git commit -m "feat: file-load error reporting; record playground in CLAUDE.md"
```
