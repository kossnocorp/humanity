# Humanity — first playable simulation

A procedural hunter-gatherer world. Observe autonomous bands, living cultures, fishing settlements, seasonal camps and migrating wildlife.

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
| B | Show/hide the entity browser (left sidebar) |
| I | Open/close the focused band's individual screen: member list left, detailed stats right |
| Alt+Up/Down | Select the previous/next browser entry |
| Alt+Left/Right | Switch Bands / Humans / Camps / Cultures / Languages |
| Enter | Open selected group/camp, or navigate to and follow a band |
| Backspace | Return from a filtered band list to its parent category |
| PageUp/PageDown / wheel over browser | Page through entries / move the selection |
| Home | Return to whole-globe distance |
| `?` (Shift+/ on US keyboards) | Open/close live population insights; Esc also closes |

### Population insights

The left browser lists active bands, remembered camps, living cultures and language
families. Click a band to center and follow it. Click a camp to center its cell and
list resident bands plus its founder (if alive elsewhere); then choose a band to
follow. Culture/language entries open their member-band lists. Language populations
are **share-weighted estimates**, since bands store language mixtures rather than
individual speakers. Camps are one record per culture/cell, with observed use and
condition shown in the browser. The list is paginated and
refreshes during simulation; Space and -/+ continue controlling time. Alt+arrows
operate the browser without rotating the globe.

The `?` screen charts population, births/deaths, lifetime death causes, and current age cohorts. It also shows how many people have low health or little food and lists recent death groups with their band's food, health, temperature and snow context. Scroll for details on smaller windows. **Time keeps flowing; Space and -/+ work in the chart view.** You can also launch with `--insights`.

Deaths are tracked as nutrition/poor-health mortality, background mortality, additional elder mortality, or conflict casualties. The shared background/health death roll is attributed proportionally to those risks using a separate deterministic random stream, preserving gameplay outcomes. Cold/snow affect food access and demand; they are not separate direct death causes in this model. Poor health can also reflect conflict injuries.

Diagnostics persist in saves with **unlimited daily history by default**. Use
`--analytics-years N` to retain N simulated years plus a baseline, or
`--no-analytics` to disable optional global history; `--analytics-unlimited` restores
unlimited recording when loading a capped/off save. The `?` dashboard also has
recording controls (off, 1/10/100 years, unlimited), separately from its display
ranges of **30/90/180 days** and **1/3/5/10/25/100 years**. Limits apply in memory
and saves. Off mode keeps current counters/state and living people's biographies;
it stops chart samples and optional event/death records. Resuming recording starts
at the current day without inventing missing history. The general journal and
recent death context additionally keep their existing 512/128-record bounds.

Open **B > Humans** to browse individual IDs, chronological ages and current roles.
Select one to follow their band and inspect personal skills, practiced days,
teaching/story gains and biography in the scrollable field notes. IDs survive band
splits. A deceased person's full record is currently removed; living biographies
can still reference them. Headless individual inspection:

```sh
cargo run -p humanity-cli --no-default-features -- --inspect saves/world.json --human 42
```

To open the viewer on a specific living individual, use
`cargo run -p humanity-cli -- --load saves/world.json --follow-human 42 --paused`.

Press **I** for the focused band's full-screen inspector. Click a member on the
left, or use **Up/Down** and **PgUp/PgDn**; wheel over the list changes selection.
The right panel shows age, health, role, working time, all personal skill/experience
and learning stats, plus the latest 30 biography events. Wheel over that panel to
scroll. Selection uses stable IDs as the simulation runs; deaths/departures and
band extinction are handled explicitly. **Space and -/+ remain live.** I, Esc or
the Close button returns to the globe without clearing focus. Launch directly with
`--band-screen --follow-band ID` (optionally `--paused`).

The `+` speed ceiling is **16,384 days/second** (64x the previous limit). Actual throughput depends on planet size and hardware; simulation work retains an 8ms frame budget to keep controls responsive.

Cream camp icons are mobile human bands, not settlements. Ochre animal icons mark occupied wildlife cells, including herds, flocks and small-animal colonies. Rivers follow the curved surface; snow lightens the ground. The header names the **northern** season; the southern hemisphere has the opposite season. Temperature overlay runs blue → red; resource overlay brown → green. Large planets use a coarse globe at orbital distance; zoom in for exact cells and rivers. `F` moves close to the next band and follows it; `--follow-band 0` starts following band 0.

### Planet arguments

For a new world, `--resolution`, `--temperature`, `--season-days`, and `--population` are required. `--load` and `--inspect` use the saved configuration instead.

| Argument | Meaning / default |
|---|---|
| `--resolution` | Icosahedron edge subdivisions, 2…632; cell count = `10 × resolution² + 2` |
| `--cell-km` | Mean neighbor-center spacing in km, 20; determines physical radius at this resolution |
| `--radius-km` | Optional physical planet radius instead of `--cell-km`; derived spacing must be 1…200 km |
| `--temperature` | Annual equatorial sea-level Celsius, solar-heating proxy; −30…60 |
| `--season-days` | Days in each of four seasons, 4…1000 |
| `--population` | Initial individual human records; memory and runtime scale with people and biographies |
| `--axial-tilt` | Seasonal temperature amplitude driver in degrees, 23.4 |
| `--ocean-fraction` | Ocean coverage target, 0.65; inland lakes are additional |
| `--rainfall` | Precipitation multiplier, 1 |
| `--seed` | Reproducible terrain and simulation seed, 42 |
| `--save` | Snapshot destination and in-game S/L path, `saves/world.json` |

### The spherical grid

The map is the dual of a subdivided icosahedron: **exactly 12 pentagons**, with every remaining cell a hexagon. Shared spherical circumcenters form the borders. There are no artificial seams or polar boundaries. The simulation uses the same graph as the 3D meshes; rivers, migrations, and tribal contacts cross all 20 base faces normally. Resource capacity accounts for actual spherical cell areas, and travel costs use great-circle neighbor distances. Hexagons are not perfectly regular or equal-area.

Resolution 28 produces 7,842 cells; resolution 128 produces 163,842. Physical radius is derived from mean spacing using `R = cell_km × sqrt(cell_count × sqrt(3) / (8π))`. The small starter planet has radius about 465 km; use `--resolution 128 --radius-km 6371` for an Earth-sized (but procedurally unrelated) planet. Increase resolution for finer geography. Lighting is an observation aid, not a simulated astronomical day/night cycle.

**Save format v10:** adds persistent flora stocks, vigor, cover and shared forage
production budgets. It includes expanded fauna, wariness, per-species population budgets,
monotonic wildlife IDs and woody browse. It also stores individual humans, chronological birth dates, personal
skills/biographies, analytics policy, local depletion, multi-season camp use,
bounded occupation histories,
condition and maintenance along with cultures, ecology and demographics. **Start a
new world: older save versions are not supported or migrated.** Geometry is rebuilt
from resolution rather than duplicated in JSON.

Food acquisition now pays for search, distance to fishing patches, and relocation
time. Repeated harvesting depletes easily reached patches within a cell; resting
places recover, even while remote regional biomass remains. Multi-year wet/dry
spells affect soil moisture, river recharge and freshwater productivity. These
constraints continue after skills mature. See `docs/research/deaths.md` for the
research, anomaly diagnosis and before/after measurements, and
`pkgs/humanity-nutrition/src/rules.rs` and `pkgs/humanity-world/src/climate.rs`
for food-search and climate tuning assumptions.

Each culture/cell has one remembered place, with fishing or residential function.
The current stay, separate occupation episodes, total occupied days and eight recent
season records distinguish repeated visits from continuous residence. Labels include
seasonal use, multi-season use, long-stay base (two continuous seasons), and observed
year-round residence (one continuous year). Learning a new season normally requires
10 occupied days within that season; its association does not imply permanent residence.

Camps lose condition without upkeep. Healthy, provisioned associated residents
maintain them; an unsupported visit updates occupancy but does not repair the camp.
From full condition, neglected residential camps disappear after roughly **3 years**,
fishing locations after roughly **6 years**. They also disappear when neither founder
nor owning culture survives. A normal annual absence can be followed by repair on
return. Attachment weakens with condition; abandoned camps free site slots and can
later be re-established with new IDs. Rates are tunable in `humanity-cultures`.

### Wildlife, fishing and cultures

`humanity-flora` now owns terrestrial vegetation. Grassland and woodland have
different grass/browse cover; overgrazing reduces vigor and future production,
while resting patches recover gradually. All local species **share one forage
production budget**, including existing residents when checking incoming groups.
Standing biomass alone no longer grants every species a separate carrying capacity.
Field notes show plant cover/vigor, seasonal supply and wildlife demand; `--inspect`
includes flora summaries and occupied-cell counts. See
[`pkgs/humanity-flora/README.md`](pkgs/humanity-flora/README.md) for ownership/rules and
[`docs/research/flora.md`](docs/research/flora.md) for the 14-year before/after audit.

Herd markers now indicate **large game**. Small animal/bird colonies have a
rabbit-shaped marker visible at close zoom or in the **Food** layer, and remain
inspectable in field notes at any zoom. Both marker types stay half the band-icon size.

**Fourteen terrestrial species** now have distinct diets, habitats, recruitment,
movement, concealment and tolerance of humans: reindeer, bison, antelope, deer,
boar, ibex, hare, rabbit, grouse, wild goose, beaver, wild horse, elk and mammoth.
`humanity-fauna` owns their data and ecology. Browsers use woody vegetation;
omnivores share fruit/seeds with human foragers. Recruitment depends on food,
density, condition and local seasons. Living populations can disperse into depleted
areas; transfers and mergers conserve animals. **Extinct species do not respawn.**

Hunting now pays for finding rare/wary animals and favors expected obtainable
returns rather than the first herd encountered. Human presence/hunting can cause
avoidance; cover and quiet refuges help survivors. Small-game biomass is derived
from actual animals, and trapping removes them instead of regenerating meat from
nothing. Wildlife markers are deduplicated per cell; field notes list all
local species, condition, wariness and available browse. `--inspect` adds
`wildlife_diagnostics` with recruitment, natural deaths, harvest and extinction status.
See [`pkgs/humanity-fauna/README.md`](pkgs/humanity-fauna/README.md) for trait data and
[`docs/research/fauna.md`](docs/research/fauna.md) for research and evaluations.

Rivers, lakes and seas retain renewable fish/shellfish biomass. Fishing competes
with hunting/gathering for labor. Bands collect craft materials, build watercraft,
preserve food and develop species-specific hunting familiarity.

Thirteen practical skills belong to individuals. **Actual work earns experience**:
hunters learn from attempts (less on failure); gathering, fishing, crafting and care
train their participants. Others hear activity-derived stories and learn more slowly,
up to 35% of the participant's skill. Demand-driven apprenticeships cap learning at
80% of the teacher's skill and spend both people's working time. Practice can exceed
those social-learning caps. No automatic mastery merely from elapsed time.

Roles respond to opportunities, member skills, care and craft needs. The band skill
summary is derived from its living members; it does not grant proficiency to everyone.
Birth dates determine actual ages; age-group counts are summaries. Healthy elders
can still work. Shared language assists inter-band transmission. Boats require
knowledge and material-built passenger capacity for water travel.

Select a cell and scroll the field-notes panel to inspect all of this. The aquatic overlay shows fish/shellfish biomass; the material overlay maps stone to red, wood to green and fiber to blue. `--inspect` includes species counts, adult knowledge, fishing totals, innovations, knowledge transfers and boat journeys.

Bands inherit one of three residential customs: **small-camp dispersal**, **balanced fission**, or **river-camp budding**. Sustained crowding produces differently sized offshoots, conserving people, supplies and knowledge. Normal thresholds are 40/70/100 people; well-provisioned fishing camps can tolerate 150, while scarcity lowers thresholds. Initial sizes no longer grow with global population. Field notes show the current threshold, pressure and parent band; chronicles record splits. Headless summaries include band sizes, custom counts and `band_fissions`. Multiple camps can share a map cell and then migrate independently. See the research and exact assumptions below.

Bands belong to shared **culture identities**. Three years of same-culture isolation can create a daughter culture; sustained peaceful contact can create a two-parent mixed culture. Five starting variants include **fishing residents** and **seasonal rounds**. These establish named fishing settlements and seasonal locations after sustained residence. Secure bands favor returning to remembered cultural sites in the appropriate local season; hunger can override attachment. Cyan houses and violet tents mark these sites on the globe; click their cell for owner, season and visit history.

**Inspect and tune the current game rules:** [`pkgs/humanity-cultures/README.md`](pkgs/humanity-cultures/README.md) describes all variants and parameters; [`src/lib.rs`](pkgs/humanity-cultures/src/lib.rs) contains the editable tables. `--inspect` includes actual living identities, variants, ancestry and locations. Saved `cultures` also retain extinct ancestors.

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

`--days` is **additional** days, not an absolute target. Headless runs default to 365 days and always save at the end; `--days 0` saves the initial world. `--report-every 0` suppresses intermediate reports. The normal viewer only saves when S is pressed. Saves contain the deterministic RNG state, individual humans and biographies, groups, language mixtures, resources, weather, counters and configured analytics. They remain inspectable versioned JSON; binary storage is deferred until measured costs justify it. Failed validation never replaces the current world or an existing save.

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
| `humanity-core` | World composition, cells, events, diagnostics and deterministic RNG; re-exports domain types |
| `humanity-humans` | Individual identities, chronological age, health, biographies and demographic lifecycle |
| `humanity-bands` | Residential groups, shared equipment, role allocation, apprenticeship and membership transfer |
| `humanity-skills` | Skill definitions, proficiency/experience, prerequisites and capped learning rules |
| `humanity-nutrition` | Foraging/hunting/fishing food calculations, resource composition, rations and consumption |
| `humanity-fauna` | Species/herd data, diets, recruitment, mortality, avoidance, dispersal and harvest accounting |
| `humanity-flora` | Plant communities/stocks, growth, seasonal forage production, degradation and recovery |
| `humanity-cultures` | Tunable cultural variant registry, camp customs, social and site parameters |
| `humanity-world` | Periodic terrain, acyclic drainage/lakes, biomes, daily weather, renewable ecology, initial seeding |
| `humanity-sim` | Daily orchestration, spatial indexes, movement, contacts and domain integration |
| `humanity-save` | Snapshot validation, atomic persistence, inspection summaries |
| `humanity-view` | Bevy 3D globe, orbit camera, UI, embedded self-authored SVG billboard icons |
| `humanity-cli` | Required arguments, headless execution, optional viewer |

Daily work is linear in cells plus spatially indexed aggregate agents; local encounters are sampled with bounded work rather than all-pairs comparisons. No entity per person or terrain cell. The grid's immutable geometry is shared on clone. Rendering keeps a bounded coarse globe and traverses only a spherical cap for detailed nearby terrain, batching it into meshes. Billboard icons use depth testing and horizon culling. The viewer budgets simulation steps per frame; requested speed may exceed achievable speed on very large worlds. Simulation is single-threaded for deterministic ordering; same-build save/resume is tested byte-for-byte equivalent at the state level.

Prototype approximations: climate and demographic hazards are stylized, not scientific forecasts. River channels use accumulated catchments, not fluid simulation. Food/wildlife stocks remain aggregate, while humans have individual records. Food/equipment and language mixtures remain shared by bands. Pregnancy, complete genealogy, age-specific disease, camp fusion, farming and later hybrid population aggregation remain future work. Fishing locations do not imply buildings or separate storage. Individualization alone is not a claim that long-run demography is calibrated.
