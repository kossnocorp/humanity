# Humanity — first playable simulation

A procedural hunter-gatherer world. You observe autonomous bands and migrating wildlife; there are no settlements or player orders yet.

## Start the game

Install stable Rust (the repository's `rust-toolchain.toml` selects it). On macOS, install Xcode Command Line Tools. The viewer needs a desktop session and a graphics driver supported by Bevy. Linux additionally needs the usual Bevy X11/Wayland development libraries; the headless build needs neither.

From the repository root:

```sh
cargo run -p humanity-cli -- \
  --width 128 --height 64 --temperature 28 \
  --season-days 90 --population 6000 --seed 42
```

The first graphics build is substantial; subsequent edits to simulation crates are much cheaper. Dependencies are optimized in development builds. Add `--paused` to start paused, or `--days 180` to pre-simulate half a year.

With mise installed, `mise run play`, `mise run simulate`, and `mise run check` provide the same standard workflows.

### Controls

| Input | Action |
|---|---|
| Space | Pause / resume |
| `-` / `+` (`=` also works) | Halve / double days per second, 0.25–256 |
| Arrow keys | Pan |
| Left-click + drag | Pan |
| Wheel / trackpad scroll | Zoom around the pointer |
| Click a hex | Inspect climate, resources, bands, demographics, languages |
| S | Atomically save to `--save` (default `saves/world.json`) |
| L | Restore that snapshot, paused |
| Tab | Biomes/snow → temperature → edible resources |
| F | Focus the next human band |
| Home | Fit the planet in the atlas |

Cream camp icons are mobile human bands, not settlements. Ochre animal icons are herds. Rivers are blue lines; snow lightens the ground. The header names the **northern** season; the southern hemisphere has the opposite season. Temperature overlay runs blue → red; resource overlay brown → green. Extremely distant zoom uses sampled regional blocks; zoom in for exact hexes and rivers.

### Planet arguments

For a new world, **all five** of `--width`, `--height`, `--temperature`, `--season-days`, and `--population` are required. `--load` and `--inspect` use the saved configuration instead.

| Argument | Meaning / default |
|---|---|
| `--width`, `--height` | Hex columns and rows; at least 24×16, even height, maximum 4 million cells |
| `--cell-km` | Neighbor-center distance in km, 20; scales resource capacity and travel time |
| `--temperature` | Annual equatorial sea-level Celsius, solar-heating proxy; −30…60 |
| `--season-days` | Days in each of four seasons, 4…1000 |
| `--population` | Initial people, 1…100 million; aggregate cohorts, not individual entities |
| `--axial-tilt` | Seasonal temperature amplitude driver in degrees, 23.4 |
| `--ocean-fraction` | Ocean coverage target, 0.65; inland lakes are additional |
| `--rainfall` | Precipitation multiplier, 1 |
| `--seed` | Reproducible terrain and simulation seed, 42 |
| `--save` | Snapshot destination and in-game S/L path, `saves/world.json` |

East/west wrap, north/south stop at the poles. This first atlas is a cylindrical approximation, not spherical geometry: equal-area hexes and latitude bands do not model polar convergence. Approximate circumference is `width × cell-km`; pole-to-pole span is `height × cell-km × sqrt(3)/2`. Climate, elevation, snow, precipitation, water, plant stocks and grazing constrain movement and survival. Some extreme configurations can be uninhabitable and are rejected at generation.

## Headless runs, resume, inspect

This command **does not compile Bevy or initialize a GPU**:

```sh
cargo run -p humanity-cli --no-default-features -- \
  --headless --width 128 --height 64 --temperature 28 \
  --season-days 90 --population 6000 --seed 42 \
  --days 720 --report-every 180 --save saves/two-years.json

# Continue for another year. Generation arguments cannot override a loaded world.
cargo run -p humanity-cli --no-default-features -- \
  --headless --load saves/two-years.json --days 360 --save saves/three-years.json

# Validate and summarize a snapshot, with recent events.
cargo run -p humanity-cli --no-default-features -- --inspect saves/three-years.json

# Open that exact simulated world, paused. S/L use the explicitly supplied path.
cargo run -p humanity-cli -- \
  --load saves/three-years.json --save saves/three-years.json --paused
```

`--days` is **additional** days, not an absolute target. Headless runs default to 365 days and always save at the end; `--days 0` saves the initial world. `--report-every 0` suppresses intermediate reports. The normal viewer only saves when S is pressed. Saves contain the entire deterministic RNG state, cohorts, language mixtures, resource stocks, weather, agents, lifetime counters and the last 512 events. They are versioned JSON; failed validation never replaces the current world or an existing save.

Inspect individual records with Python:

```sh
python3 - <<'PY'
import json
w = json.load(open('saves/two-years.json'))
print('day:', w['day'], 'totals:', w['totals'])
t = max(w['tribes'], key=lambda t: sum(t['people']['female'] + t['people']['male']))
print('largest band:', t)
print('its cell:', w['cells'][t['cell']])
print('recent events:', w['events'][-5:])
PY
```

For a larger workload use `--width 512 --height 256 --population 1000000`. Prefer `--release` for extended runs. Oversupplying population can exhaust wildlife and trigger starvation/conflict; population is not guaranteed to survive or grow. Snapshots are intentionally inspectable rather than compressed; very large worlds produce large files.

## Development

```sh
cargo fmt --all --check
cargo clippy --workspace --all-targets -- -D warnings
cargo test --workspace

# Fast, renderer-free test cycle:
cargo test -p humanity-core -p humanity-world -p humanity-sim -p humanity-save

# Render smoke test: capture after startup and exit; requires a display.
cargo run -p humanity-cli -- \
  --load saves/two-years.json --paused --screenshot saves/preview.png
```

### Crate boundaries

| Crate | Responsibility |
|---|---|
| `humanity-core` | Compact state, units, hex topology, serializable deterministic RNG |
| `humanity-world` | Periodic terrain, acyclic drainage/lakes, biomes, daily weather, renewable ecology, initial seeding |
| `humanity-sim` | Daily phases, sparse spatial indexes, herd/tribe decisions, hunting, contact, demography |
| `humanity-save` | Snapshot validation, atomic persistence, inspection summaries |
| `humanity-view` | Bevy map, camera, UI, embedded self-authored SVG icons |
| `humanity-cli` | Required arguments, headless execution, optional viewer |

Daily work is linear in cells plus spatially indexed aggregate agents; local encounters are sampled with bounded work rather than all-pairs comparisons. No entity per person or terrain hex. Rendering culls terrain to the viewport, bounds displayed detail, and batches terrain into meshes. The viewer budgets simulation steps per frame; requested speed may exceed achievable speed on very large worlds. Simulation is single-threaded for deterministic ordering; same-build save/resume is tested byte-for-byte equivalent at the state level.

Prototype approximations: climate and demography are stylized, not scientific forecasts. Seas are shallow ocean margins. River channels use accumulated climatological catchments, not daily fluid simulation. Cell plants, fruit/seeds, small game and grazing are aggregate stocks; herds are reindeer, bison or antelope. Language families mix through peaceful contact. Demography uses female/male child/adult/elder cohorts, with derived labor roles. Events resolve using stored probabilities; conflict events record the winner's combat probability. No boats, farming, settlements, band fission, individual genealogy, or historical language naming yet.
