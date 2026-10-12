# Humanity — first playable simulation

A procedural hunter-gatherer world. You observe autonomous bands and migrating wildlife; there are no settlements or player orders yet.

## Start the game

Install stable Rust (the repository's `rust-toolchain.toml` selects it). On macOS, install Xcode Command Line Tools. The viewer needs a desktop session and a graphics driver supported by Bevy. Linux additionally needs the usual Bevy X11/Wayland development libraries; the headless build needs neither.

From the repository root:

```sh
cargo run -p humanity-cli -- \
  --resolution 28 --temperature 28 \
  --season-days 90 --population 6000 --seed 42
```

The first graphics build is substantial; subsequent edits to simulation crates are much cheaper. Dependencies are optimized in development builds. Add `--paused` to start paused, or `--days 180` to pre-simulate half a year.

With mise installed, `mise run play`, `mise run simulate`, and `mise run check` provide the same standard workflows.

### Controls

| Input | Action |
|---|---|
| Space | Pause / resume |
| `-` / `+` (`=` also works) | Halve / double days per second, 0.25–256 |
| Arrow keys | Rotate the globe |
| Left-click + drag | Rotate freely, including across the poles |
| Wheel / trackpad scroll | Zoom around the pointer |
| Click a surface cell | Inspect climate, resources, bands, demographics, languages |
| S | Atomically save to `--save` (default `saves/world.json`) |
| L | Restore that snapshot, paused |
| Tab | Biomes/snow → temperature → land food → aquatic food → craft materials |
| Wheel over field notes | Scroll resources, wildlife, band knowledge and equipment |
| F | Follow the next human band as it moves; arrows/drag/click/Home release it |
| Esc | Unfollow and clear the active selection; also close population insights |
| Home | Return to whole-globe distance |
| `?` (Shift+/ on US keyboards) | Open/close live population insights; Esc also closes |

### Population insights

The `?` screen charts population, births/deaths, lifetime death causes, and current age cohorts. It also shows how many people have low health or little food and lists recent death groups with their band's food, health, temperature and snow context. Scroll for details on smaller windows. **Time keeps flowing; Space and -/+ work in the chart view.** You can also launch with `--insights`.

Deaths are tracked as nutrition/poor-health mortality, background mortality, additional elder mortality, or conflict casualties. The shared background/health death roll is attributed proportionally to those risks using a separate deterministic random stream, preserving gameplay outcomes. Cold/snow affect food access and demand; they are not separate direct death causes in this model. Poor health can also reflect conflict injuries.

Diagnostics persist in saves: the last 720 daily samples, lifetime cause totals, and 128 recent death groups. Spherical saves lacking diagnostics start history when loaded and label previous deaths **unrecorded**, without inventing causes or past charts. `--inspect` and headless JSON summaries also include cause totals.

Cream camp icons are mobile human bands, not settlements. Ochre animal icons are herds. Rivers follow the curved surface; snow lightens the ground. The header names the **northern** season; the southern hemisphere has the opposite season. Temperature overlay runs blue → red; resource overlay brown → green. Large planets use a coarse globe at orbital distance; zoom in for exact cells and rivers. `F` moves close to the next band and follows it; `--follow-band 0` starts following band 0.

### Planet arguments

For a new world, `--resolution`, `--temperature`, `--season-days`, and `--population` are required. `--load` and `--inspect` use the saved configuration instead.

| Argument | Meaning / default |
|---|---|
| `--resolution` | Icosahedron edge subdivisions, 2…632; cell count = `10 × resolution² + 2` |
| `--cell-km` | Mean neighbor-center spacing in km, 20; determines physical radius at this resolution |
| `--radius-km` | Optional physical planet radius instead of `--cell-km`; derived spacing must be 1…200 km |
| `--temperature` | Annual equatorial sea-level Celsius, solar-heating proxy; −30…60 |
| `--season-days` | Days in each of four seasons, 4…1000 |
| `--population` | Initial people, 1…100 million; aggregate cohorts, not individual entities |
| `--axial-tilt` | Seasonal temperature amplitude driver in degrees, 23.4 |
| `--ocean-fraction` | Ocean coverage target, 0.65; inland lakes are additional |
| `--rainfall` | Precipitation multiplier, 1 |
| `--seed` | Reproducible terrain and simulation seed, 42 |
| `--save` | Snapshot destination and in-game S/L path, `saves/world.json` |

### The spherical grid

The map is the dual of a subdivided icosahedron: **exactly 12 pentagons**, with every remaining cell a hexagon. Shared spherical circumcenters form the borders. There are no artificial seams or polar boundaries. The simulation uses the same graph as the 3D meshes; rivers, migrations, and tribal contacts cross all 20 base faces normally. Resource capacity accounts for actual spherical cell areas, and travel costs use great-circle neighbor distances. Hexagons are not perfectly regular or equal-area.

Resolution 28 produces 7,842 cells; resolution 128 produces 163,842. Physical radius is derived from mean spacing using `R = cell_km × sqrt(cell_count × sqrt(3) / (8π))`. The small starter planet has radius about 465 km; use `--resolution 128 --radius-km 6371` for an Earth-sized (but procedurally unrelated) planet. Increase resolution for finer geography. Lighting is an observation aid, not a simulated astronomical day/night cycle.

**Save format v3:** knowledge, equipment, aquatic stocks and material resources persist. Spherical v2 saves upgrade deterministically on load: existing people, geography, events and RNG are preserved, new resources and starting repertoires are initialized. Their future trajectory changes under the new rules. Cylindrical v1 saves still require a new planet. Geometry is rebuilt from resolution rather than duplicated in JSON.

### Wildlife, fishing and cultures

Six migrating land-animal types have distinct habitat preferences, reproductive rates, hunting difficulty and meat yields. Rivers, lakes and seas carry renewable fish and shellfish biomass. Fishing competes with hunting/gathering for labor. Bands collect wood, fiber, toolstone and hides, craft watercraft, preserve food, and develop species-specific hunting familiarity.

Thirteen practical traditions spread through adult/elder teaching, children's apprenticeship, local experimentation and peaceful inter-band contact. Shared language makes transmission more likely; demonstration can occasionally cross language barriers. Boats require both knowledge and enough material-built passenger capacity; they enable short water crossings and faster river travel.

Select a cell and scroll the field-notes panel to inspect all of this. The aquatic overlay shows fish/shellfish biomass; the material overlay maps stone to red, wood to green and fiber to blue. `--inspect` includes species counts, adult knowledge, fishing totals, innovations, knowledge transfers and boat journeys.

See [PREHISTORY.md](PREHISTORY.md) for archaeological examples, sources, exact mechanisms and modelling boundaries.

## Headless runs, resume, inspect

This command **does not compile Bevy or initialize a GPU**:

```sh
cargo run -p humanity-cli --no-default-features -- \
  --headless --resolution 28 --temperature 28 \
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

For a larger workload use `--resolution 128 --population 1000000`. Prefer `--release` for extended runs. Oversupplying population can exhaust wildlife and trigger starvation/conflict; population is not guaranteed to survive or grow. Snapshots are intentionally inspectable rather than compressed; very large worlds produce large files.

## Development

```sh
cargo fmt --all --check
cargo clippy --workspace --all-targets -- -D warnings
cargo test --workspace

# Fast, renderer-free test cycle:
cargo test -p humanity-grid -p humanity-core -p humanity-world -p humanity-sim -p humanity-save

# Render smoke test: capture after startup and exit; requires a display.
cargo run -p humanity-cli -- \
  --load saves/two-years.json --paused --screenshot saves/preview.png
```

### Crate boundaries

| Crate | Responsibility |
|---|---|
| `humanity-grid` | Geodesic topology, spherical geometry, areas, neighbor queries and picking |
| `humanity-core` | Compact state, units, serializable deterministic RNG |
| `humanity-world` | Periodic terrain, acyclic drainage/lakes, biomes, daily weather, renewable ecology, initial seeding |
| `humanity-sim` | Daily phases, sparse spatial indexes, herd/tribe decisions, hunting, contact, demography |
| `humanity-save` | Snapshot validation, atomic persistence, inspection summaries |
| `humanity-view` | Bevy 3D globe, orbit camera, UI, embedded self-authored SVG billboard icons |
| `humanity-cli` | Required arguments, headless execution, optional viewer |

Daily work is linear in cells plus spatially indexed aggregate agents; local encounters are sampled with bounded work rather than all-pairs comparisons. No entity per person or terrain cell. The grid's immutable geometry is shared on clone. Rendering keeps a bounded coarse globe and traverses only a spherical cap for detailed nearby terrain, batching it into meshes. Billboard icons use depth testing and horizon culling. The viewer budgets simulation steps per frame; requested speed may exceed achievable speed on very large worlds. Simulation is single-threaded for deterministic ordering; same-build save/resume is tested byte-for-byte equivalent at the state level.

Prototype approximations: climate and demography are stylized, not scientific forecasts. Seas are shallow ocean margins. River channels use accumulated climatological catchments, not daily fluid simulation. Cell plants, fruit/seeds, small game and grazing are aggregate stocks; herds are reindeer, bison or antelope. Language families mix through peaceful contact. Demography uses female/male child/adult/elder cohorts, with derived labor roles. Events resolve using stored probabilities; conflict events record the winner's combat probability. No boats, farming, settlements, band fission, individual genealogy, or historical language naming yet.
