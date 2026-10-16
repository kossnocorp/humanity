# Game model and follow-up work

## Current core entities

| Entity | Meaning |
|---|---|
| World | Planet configuration, spherical grid, time, deterministic RNG, entities and history |
| Cell | Elevation, biome, drainage/river, weather and resource stocks |
| Human | Stable identity, chronological birth date, sex, personal health, role, independent skills/experience and biography |
| Band (`Tribe`) | Residential group of individual members, shared food/equipment, language/culture affiliation and derived capability summaries |
| Demography | Living individual records with derived sex/life-stage counts; chronological birthdays |
| Specialists | Derived daily hunter/gatherer/fisher/healer/guide assignments responding to skills and opportunities |
| Culture | Named shared identity with variant, founding date and up to two parent identities |
| Cultural site | One remembered place per culture/cell: fishing or residential function, multiple seasons of use, occupation history, condition and upkeep |
| Wildlife population / species | Aggregate herd, flock or colony with condition, travel and wariness; 14 species with distinct diets and life histories |
| Vegetation / forage | Plant-community stocks, cover/vigor, seasonal production and a shared food budget for all wildlife consumers |
| Biome niches | Dry belts and humid tropics; steppe/scrub/marsh/swamp communities, terrain travel costs and camel/tapir/water-buffalo specialists |
| Language mixture | Family shares within a band; overlap affects mutual understanding |
| Skills / Knowledge | Personal proficiency/experience in 13 skills; derived band means and shared prey familiarity |
| Equipment | Portable wood, fiber, stone, hides and boat passenger capacity |
| Resources | Plants, fruit, small game, grazing, aquatic biomass and crafting resources |
| Event / totals / insights | Recent outcome journal, lifetime counters, population/mortality diagnostics |

### Cultural rule layers

- **Identity:** the specific community of bands, with cultural ancestry.
- **Variant:** dispersed foragers, cooperative foragers, river networks, fishing residents, seasonal rounds.
- **Camp tradition:** small-party dispersal, balanced fission or river budding.
- Rules and comments: `pkgs/humanity-cultures/src/lib.rs`; tuning guide: its `README.md`.

### Existing mechanics

- Seasonal weather, snow, moisture and renewable resources.
- Wildlife habitat selection, migration, grazing, reproduction and mortality.
- Gathering, hunting, fishing, consumption and spoilage.
- Material collection, watercraft construction/wear, preservation, clothing and care.
- Resource-seeking travel, river routes, short crossings and remembered-site returns.
- Teaching, apprenticeship, experimentation, skill transmission and loss.
- Peaceful exchange, material sharing, language mixing and conflict.
- Births, aging, health recovery, mortality and extinction.
- Band fission, cultural divergence/mixing, site founding and site lifetime.

### Recorded event types

`Hunt`, `HuntFailed`, `Fishing`, `Conflict`, `Exchange`, `KnowledgeExchange`,
`Migration`, `WaterCrossing`, `Birth`, `Death`, `Extinction`, `Innovation`,
`BandFission`, `CultureDivergence`, `CultureMixing`, `SiteFounded`, `SiteAbandoned`.

Events record outcomes; simulation systems execute mechanics. Each event includes day,
cell, band, optional other participant, amount and probability field. Recent history
retains 512 events; lifetime totals and population diagnostics are separate.

### Code map

- `pkgs/humanity-core/src/lib.rs`: world composition, cells, events and domain re-exports.
- `pkgs/humanity-humans/src/{lib,demography}.rs`: individuals, biographies and chronological lifecycle.
- `pkgs/humanity-bands/src/{lib,roles,learning,membership}.rs`: groups, work allocation, apprenticeship/stories and fission.
- `pkgs/humanity-skills/src/{lib,learning}.rs`: skill definitions, personal experience and capped learning rules.
- `pkgs/humanity-nutrition/src/{foraging,hunting,fishing,consumption,stocks,rules}.rs`: food acquisition, stocks and consumption.
- `pkgs/humanity-fauna/src/{lib,species,ecology,hunting}.rs`: animal data/traits, resource-limited ecology, avoidance, dispersal, encounter and harvest.
- `pkgs/humanity-world/src/wildlife.rs`: world/fauna input/output adapter and derived small-game biomass.
- `pkgs/humanity-flora/src/{stocks,rules,growth}.rs`: terrestrial plant data, community profiles, growth and productive forage limits.
- `pkgs/humanity-world/src/flora.rs`: world biome/weather adapter for flora.
- `pkgs/humanity-cultures/src/lib.rs`: shared cultural identities, customs and social tuning.
- `pkgs/humanity-core/src/sites.rs`: places, occupation histories and condition.
- `pkgs/humanity-core/src/insights.rs`: population/mortality records.
- `pkgs/humanity-sim/src/lib.rs`: daily phases, movement, feeding, contacts, demography.
- `pkgs/humanity-sim/src/subsistence.rs`: crafting, fishing and practical learning.
- `pkgs/humanity-sim/src/social.rs`: identity change, camp fission and remembered sites.
- `pkgs/humanity-world/src/lib.rs`: generation, climate and ecological renewal.
- `pkgs/humanity-world/src/climate.rs`: interannual hydrology tuning formerly misplaced in cultures.

## Active direction: domain crates, aging, roles and skills

The first individual implementation and four domain crates are shipped in **v8**.
Checked items below are implemented; unchecked items remain follow-up work.
JSON saves remain inspectable and require fresh worlds. No migrations.

Individuals practice through actual work; others learn less from those activities'
stories. Teachers cap instruction at 80% of their own skill; stories at 35%.
The browser's **Humans** tab and `--inspect SAVE --human ID` expose personal records.
**I** opens a live two-column band screen: member list on the left, individual
stats/skills and recent biography on the right; mouse/keyboard selection and scrolling.
Analytics defaults to unlimited; configurable years/off leave living biographies intact.

### Domain ownership and crate boundaries

| Crate | Intended responsibility |
|---|---|
| `humanity-humans` | Individual member records, identity, sex, chronological age, health, demographic lifecycle and personal skill/role state; later aggregate population representation and conversion invariants |
| `humanity-bands` | Residential group state, membership, shared provisions/equipment, collective decisions and labor demand, language/culture affiliation, fission/fusion and member transfers |
| `humanity-skills` | Skill definitions, proficiency/experience, practice progression, prerequisites, teaching/apprenticeship and transmission rules |
| `humanity-nutrition` | Food acquisition and nutrition: foraging, hunting, fishing, accessible resource stocks/depletion, work/travel budgets, food requirements, consumption, preservation/spoilage and relevant tuning/data |
| `humanity-fauna` | Animal populations, species traits/diets, reproduction, mortality, human avoidance, dispersal and physical animal harvest |
| `humanity-flora` | Terrestrial plant stocks, community cover, growth, forage production and overuse/recovery state |
| `humanity-cultures` | Cultural identities/customs/variants and social preferences; not generic ecology or human learning mechanics |
| `humanity-world` | Planet generation, weather/climate, hydrology and habitat dynamics; invokes food-resource renewal where appropriate |
| `humanity-core` | Shared world composition, identifiers and cross-domain records; reduce its existing ownership of unrelated domain types |
| `humanity-sim` | Daily orchestration, interactions and event recording; delegate domain algorithms to their owning crates |
| `humanity-save` / `humanity-view` | Persistence/validation and presentation; neither owns simulation rules |

- [x] Create `humanity-nutrition` with `foraging.rs`, `hunting.rs`, `fishing.rs`
  and supporting modules for stocks, consumption/storage and tuning as needed.
  Extract actual mechanics and their structs/data, not merely rename the existing
  constants file or introduce wrappers around code still owned by the simulator.
- [x] Remove `humanity-cultures/src/ecology.rs`. Move food/search/depletion rules
  to nutrition; put interannual climate/hydrology rules with world/environment
  ownership rather than treating all contents of that file as nutrition.
- [x] Create `humanity-humans` and move individual and demographic state out of
  `humanity-core`; create `humanity-bands` separately for group state and membership.
  Define ownership of personal versus shared equipment, languages and membership
  explicitly; keep cultural identities distinct from a person's/group's affiliation.
- [x] Create `humanity-skills` and move `Skill`, knowledge/proficiency state,
  practice, teaching and learning algorithms out of core/simulation subsistence.
- [x] Establish an acyclic dependency graph before extraction: low-level skills
  and human types must not depend on the world or simulator. Pass resource,
  environmental and population inputs across domain boundaries; avoid a
  `core -> nutrition -> core` cycle and avoid moving the whole world into nutrition.
- [x] Retain renderer-independent, seeded headless execution and fast incremental
  builds. Document every new data field and tuning assumption in its owning crate.
- [x] Update workspace dependencies, the code map, `HUMANS.md`, crate READMEs,
  snapshot validation and viewer inspection as ownership changes.

### Chronological aging and human representation

- [x] Replace random transitions between three age stages with chronological
  aging. Birth cohorts advance by actual elapsed simulated years; newborns must
  not randomly become working adults immediately, nor newly mature adults become
  elders immediately. Children/adults/elders remain useful UI summaries.
- [x] Initial representation decision: **individual member records**, with stable
  identity and independent chronological age, role and skill history. The user
  favors personal stories and detailed small-scale play, with aggregation later
  as population and social composition grow. No individual render entity is required.
- [ ] Build the individual path first and benchmark thousands/tens/hundreds of
  thousands of people. Measure simulation days/second, frame-time budget, memory,
  save size and loading; do not impose a guessed universal population cutoff.
  Individual path and 6,000-person century / 100,000-person short-run checks are
  complete; further scale profiling remains. See `docs/research/individuals-validation.md`.
- [ ] Keep a boundary for a later aggregate population representation. Group
  systems should query population capabilities, labor, needs and demographics
  rather than assume every resident always has an individually updated record.
  This boundary must not prevent inspecting individual members in detailed mode.
- [ ] Design hybrid aggregation as a separate milestone: aggregate selected
  groups/regions while retaining detailed groups and notable people where useful.
  Apply thresholds with hysteresis and an explicit simulation policy rather than
  silently changing the whole world's rules when a single population count is crossed.
- [ ] Preserve distributions, not just means, when aggregating: age/sex, health,
  skill combinations and novice/expert composition, fertility eligibility, roles,
  culture/language and relevant household structure. Avoid representing one expert
  and many novices as many equally competent workers. Bound combinatorial growth.
- [ ] Define conversion semantics and tests before implementing hybrid mode:
  conserve people, assets and food; preserve demographic/skill distributions within
  declared tolerances; prevent notable people being counted twice; persist mode and
  deterministic RNG state. Deaggregation cannot reconstruct discarded biographies:
  distinguish retained identities from newly sampled representative members.
- [ ] Compare detailed and aggregate modes statistically over matched scenarios;
  require deterministic replay within a mode/conversion schedule, not byte-identical
  trajectories across different demographic approximations.
- [ ] Tie fertility, participation in work, learning and eventual age-dependent
  mortality to the actual age representation. Preserve population, sex counts and
  learned skills through aging, deaths, moves, band fission and membership changes.
- [ ] Make age-appropriate food requirements and productive/care contributions
  explicit. Account for maternal costs when distinguishing child rations; do not
  assume all healthy people stop contributing on their fiftieth birthday.

### Flexible roles and RPG-like practical skills

- [x] Replace fixed adult role percentages with assignments responding to available
  work, expected returns, food needs, member ability, health and extra hands needed.
  Roles are current jobs, not permanent human classes or global culture unlocks.
- [x] Give represented members differentiated skill levels/proficiency. Actual
  practice grants inspectable experience/progression, including bounded learning
  from unsuccessful attempts; merely existing must not raise every skill to mastery.
- [x] Let experienced members teach novices when additional workers are needed.
  Teaching and apprenticeship consume a real time/work budget; account for the
  novice's lower productivity and the teacher's diverted effort.
- [x] **Cap teaching at a configurable percentage of the teacher's own skill.**
  A low-skilled hunter cannot teach an apprentice to mastery. Apply the cap to
  each teacher/learner transfer; do not lower an already more-skilled learner.
  Define the percentage in `humanity-skills` rather than invent an undocumented
  fixed value. Practice/experimentation may progress beyond what a teacher can teach.
- [x] Preserve skill diversity, prerequisites, age/experience effects and loss of
  expertise when knowledgeable members die or depart. Eliminate the current
  within-band teaching rule's ability to drive learners toward full proficiency
  regardless of the teacher's own level.
- [ ] Distinguish practical proficiency/experience from cultural transmission and
  knowledge of a technique. Inspect role demand, assignments, skill levels,
  teacher/learner relationships or aggregates, and learning limits in saves/UI.

### Implementation and verification sequence

1. Use the chosen individual representation and settle dependency boundaries;
   extract domain crates, including `humanity-bands`, with
   behavior-preserving checks before introducing new demographic behavior.
2. Implement chronological aging and conservation of human/skill state.
3. Implement practice progression and teacher-relative caps, then connect flexible
   roles and demand-driven apprenticeship to the shared work budget.
4. Connect nutrition's expected returns to allocation/movement decisions; retain
   local depletion and actual resource costs rather than restore free harvesting.
5. Add configurable analytics retention/off mode alongside individual histories;
   inspect matched-seed century-scale runs for replacement, recovery and extinction
   frequency; update research notes with measured effects and remaining limitations.
6. Measure the detailed implementation; introduce optional hybrid aggregation when
   scale warrants it. Defer binary saves until actual persistence costs become a
   problem. Later settlements/institutions may compose multiple groups; do not
   overload `humanity-humans` with all social systems.

- [x] Test age boundaries and elapsed-time behavior, fertility eligibility and
  population/skill conservation across demographic and membership operations.
- [x] Test weak-teacher caps, no teaching gains above the cap, learning from stronger
  teachers, practice beyond a teaching ceiling, and conservation of working time.
- [x] Test reassignment when opportunities change, novice productivity, and support
  for bands that need extra hands rather than a fixed fraction of every profession.
- [x] Run formatting, relevant/workspace tests and Clippy; verify deterministic
  save/resume, inspect saved states and exercise the updated viewer.

### Experimentation, analytics retention and persistence

**Current priority: explore the model and individual stories.** Higher RAM use,
larger saves and imperfect throughput are acceptable during experimentation.
Measurements should inform later optimization, not prematurely constrain the
individual-human design or force aggregation.

- [x] Make analytics storage configurable with three explicit modes:
  **unlimited (default)**, **retain the last N simulated years**, and **off**.
  Replace the current hard-coded 100-year population-history limit. Use the world's
  calendar to convert years to days; distinguish off from an unlimited setting.
- [x] Apply retention in memory as well as in saves. Define which optional chart
  samples, diagnostic event/death records and other analytics collections each
  policy covers; do not leave uncapped parallel histories defeating the RAM limit.
  Retain the boundary sample needed for correct cumulative-counter differences
  when displaying a capped time window.
- [x] When analytics are off, skip collecting optional historical analytics rather
  than merely hiding charts or omitting them when saving. Keep **current-state
  inspection and histories of existing humans** available. Operational simulation
  state, human biographies and optional world analytics need explicit ownership
  boundaries; disabling analytics must not disable mechanics or erase living
  people's stories.
- [x] Keep existing-human histories independent of the analytics year cap. Define
  the separate policy for deceased-human archives before retaining an unbounded
  archive by accident; living members may retain references to deceased people
  without requiring complete archived records for everyone who ever lived.
  Implemented policy: remove deceased full records, retain referenced IDs in living biographies.
- [x] Expose analytics mode/year limit through configuration and controls suitable
  for headless experiments and the viewer; persist the policy in saves. When a
  limit is reduced or analytics disabled, release optional retained history.
  Re-enabling recording starts a fresh baseline without reconstructing missing years.
- [x] Make charts report disabled recording, available coverage and gaps honestly.
  Keep current population, member inspection and simulation counters usable even
  when no time series exists. Storage policy must not consume gameplay RNG or
  change demographic, learning or ecological outcomes.
- [x] Test unlimited retention, exact year-window eviction/baselines, off mode,
  policy changes, living-human history preservation, and deterministic resume
  under each policy. Compare gameplay state across analytics modes.
- [ ] Keep the current inspectable save format for now. **Switch to a suitable
  binary format only when measured save size, loading/saving time or memory costs
  become a real problem.** Evaluate formats against the resulting human/skill data
  model then, rather than selecting one speculatively. Preserve a current-state
  inspection/export path when that switch is eventually made.
- [ ] Track memory by live human state, living-human history, world state and
  optional analytics separately. Track serialization peak memory as well as file
  size. A retention setting limits historical years, not necessarily total bytes;
  record that distinction when evaluating large-population experiments.

Analytics retention/off modes are implemented; binary conversion remains deferred.
Old-save compatibility and migrations remain explicitly unnecessary.

## Population resilience: research and remaining work

Research: [`docs/research/deaths.md`](docs/research/deaths.md) and
[`docs/research/population-balance.md`](docs/research/population-balance.md).

- [x] Investigate the disappearance of nutrition/poor-health deaths after skill
  maturation; introduce search/depletion, relocation costs and hydrological variation.
- [x] Audit subsequent long-run decline without changing parameters. Three seeds
  decline to 45–64 people by year 300; seed 42 becomes extinct around year 403.
  This is a replacement deficit, not just a terminal unlucky event.
- [ ] Address the age/role/skill issues above before treating higher births or lower
  mortality constants as the solution. Correcting cohort semantics can affect
  replacement even at unchanged rate coefficients.
- [ ] Investigate need-based inter-band sharing, recruitment and flexible camp
  membership to help recover from worker losses. Existing within-band food pooling
  and one-way fission do not provide all these mechanisms.
- [x] Introduce resource-limited fauna budding/recolonization from surviving
  populations, realistic search penalties and conservative animal accounting.
  Fresh v9 worlds contain 14 species; no ongoing generation or extinction floor.
  See `docs/research/fauna.md` for baseline diagnosis and multi-seed evaluations.
- [x] Correct v9 overexpansion with `humanity-flora`: replace independent species
  pasture allowances with shared production/reserve budgets, account for all
  competing diets and incoming migrants, remove the empty-cell capacity floor,
  differentiate forest understory from grassland, and model overgrazing/rest.
  v10 also distinguishes large-game markers from small colonies. Validation and
  remaining calibration limits are in `docs/research/flora.md`.
- [ ] Extend guild-level flora with named plant species, soil/seed dynamics and
  succession where useful; calibrate productive-flow and reserve assumptions across
  climates without reinstating global herd quotas or unconditional food renewal.
- [ ] Broaden fauna calibration across climates and densities; consider explicit
  predators, detailed animal age/sex structure, mate-seeking and longer migrations.
- [ ] Replace appropriate portions of residual background mortality with explicit,
  evidence-informed mechanisms eventually. Do not simply stack new disease deaths
  on top of the existing placeholder hazard.
- [ ] Evaluate recovery and extinction across habitats and seeds, including poor
  environments where extinction is reasonable. **Occasional stochastic extinction
  is acceptable; consistently inevitable extinction in viable worlds is not.**
  No guaranteed-survival population floor, mandatory growth, or cause-share quotas.

The current 1.8% annual background hazard is an age-independent placeholder, not
an infection simulation. Measured nutrition/poor-health shares are not established
prehistoric targets. Compare age-specific survival, fertility, food access and
causes per person-year together with percentages.

## Camp rework shipped (2026-10-13)

- One place per culture/cell, with multiple learned local-season associations.
- Fishing/residential function separate from observed occupation pattern.
- Current/longest stay, episodes, lifetime occupied days and eight recent seasons of
  occupied days/person-days; a visit in one season no longer overwrites another's history.
- Daily decay and upkeep: roughly 3 unmaintained years for residential camps, 6 for
  fishing locations. Healthy/provisioned associated residents restore condition;
  unsupported visits do not. Condition scales attachment. Forgotten camps disappear,
  emit `SiteAbandoned`, free site slots and receive new IDs if later re-founded.
- Browser/field notes expose condition and observed use. Four seasons of visits
  alone do not label a place permanent; continuous stays are tracked separately.
- **Fresh v5 worlds required.** Per user direction, old-save migration/compatibility
  was removed. The investigation below describes the previous v4 simulation.

Still pending: explicit sub-cell trips and full net-return movement calibration,
shared physical places across cultures, and classification hysteresis if needed.

## Seasonal locations: investigation (2026-10-12, pre-rework)

Examined `saves/world.json` at **day 81,625**, year 227, 90 days/season,
20 km mean cell spacing. Source SHA-256:
`e99891e4e9b7e1797d7fd32a8c66d74d82f411d0bc7e250ec5f50ccce7d417e7`.

### Observations

- 6,918 people, 207 bands, 428 site records: 377 seasonal and 51 fishing.
- Those 377 seasonal records represent **166 culture/cell pairs**, across 90 cells.
  97 pairs have multiple season records; 45 have all four. Only 24 of those
  four-season pairs have a currently living owning culture.
- 123 seasonal records have extinct owning cultures but a living founder band,
  consistent with the requested lifetime rule. Founder visits update these too.
- 62 bands currently use SeasonalRounds. Their current residence-duration median
  is 5.5 days, maximum 143 days; three have stayed at least 90 days, none 360 days.
- One fishing-resident band's current residence counter is 1,970 days.
- 202/207 bands have at least 30 stored food-days/person; 206/207 have health ≥.99.
  This suggests ample provisioning in this particular mature surviving population,
  not proof that every biome or starting configuration is overproductive.

### One-year copied-save continuation

Advanced a copy daily through day 81,985, without replacing the user's save.
Counted whether at least one **owning-culture** band occupied each original
culture/cell pair each day (founders now in another culture do not count here).

| Culture / cell | Occupied days / 360 | Longest observed continuous run | Occupation episodes within window |
|---|---:|---:|---:|
| 117 / 2847 | 331 | 226 | 6 |
| 130 / 2056 | 342 | 129 | 11 |
| 131 / 6387 | 226 | 18 | 39 |
| 84 / 5776 | 335 | 128 | 16 |

85 of the 166 original seasonal pairs were occupied during the window; none was
occupied on every day. Across all bands there were 25,715 observed cell moves,
including 1,417 immediate next-day returns to the prior cell. These results show
both long multi-season stays and repeated short occupations. Counts are at cell
resolution, not evidence of individual households occupying one exact campsite.

### Causes in the pre-rework implementation

1. Site identity includes `(culture, cell, kind, season)`, so crossing a season
   boundary can found another record without moving. The residence counter is not
   season-specific: founding in a new season does not require 20 days in that season.
2. Occupation by the culture **or founder** refreshes every corresponding record's
   `last_visited`, regardless of season. This timestamp cannot establish seasonal use.
3. Four seasonal records at one cell make that cell eligible for attachment all year.
   Repeated records also consume the 12-site-per-culture limit and overlap visually.
4. Movement scores standing food stocks capped at 25, rather than expected net
   harvest per worker or locally increasing foraging travel. At an eligible home,
   attachment rewards staying and penalizes movement away; crowding is measured in
   bands rather than people and includes the current band in the origin bucket.
   These are candidates for both excessive attachment and oscillatory movement;
   the copied run alone does not isolate their individual causal contributions.
5. At 20 km cell spacing, staying within one cell can represent moving camps within
   a substantial territory. Current state has no sub-cell camp positions or
   residential-versus-logistical foraging distinction.

### Refactor checklist

- [x] Give a remembered place one stable identity; represent multiple seasons of
  use on that place instead of creating a place per season. Initially deduplicate
  per culture/cell; consider shared physical places with separate cultural associations
  so ancestral ownership and founder access remain explicit.
- [x] Separate **function** (residential/fishing place; dedicated task camps remain future work)
  from **observed occupation pattern** (seasonal, multi-season, near-year-round).
  Culture should bias mobility, not permanently dictate a site's classification.
- [x] Track season-specific occupied days, distinct occupation episodes,
  consecutive occupancy, absences, residents and recent person-days. Keep historical
  use separate from recent use so centuries of visits do not imply current permanence.
- [x] Require new saves instead of reconstructing legacy occupancy (user decision).
- [ ] Base descriptive labels on observed duration/repetition, with minimum evidence
  and hysteresis. Four visited seasons alone must not imply continuous settlement.
- [ ] Measure and balance expected **net foraging returns**, localized depletion,
  renewal, accessible daily range, water, storage and seasonal resource pulses.
  Distinguish short foraging trips from moving the whole residential camp.
  Implemented 2026-10-14: density-dependent acquisition, coarse accessible-patch
  depletion/rest, single-patch fishing with distance costs, relocation work costs,
  and multi-year hydrological variation. Full net-return movement scoring remains.
- [ ] Revisit population-based crowding, self-counting and switching costs to reduce
  artificial A→B→A moves. Test attachment strengths separately from ecological renewal.
  Population weighting and self-exclusion are implemented; systematic jitter and
  attachment calibration remain. See `docs/research/deaths.md` for the mortality audit.
- [ ] Compare across climates, densities and multiple seeds: residence durations,
  distances/move, return frequency, seasonal occupancy, food budgets and mortality.
  Permit prolonged residence where supported instead of forcing quarterly migration.

### Research informing the recommendation

- Venkataraman et al. (2017), [Hunter-gatherer residential mobility and the marginal
  value of rainforest patches](https://doi.org/10.1073/pnas.1617542114): Batek data
  connect moves to declining resource returns **before complete depletion**. This
  is an informative mechanism, not a universal timetable for all hunter-gatherers.
- Clark et al. (2025), [Exploring proxies for occupation intensity in hunter-gatherer
  settlement systems](https://doi.org/10.1371/journal.pone.0333870): distinguish group
  size, stay duration and reoccupation frequency; repeated use is not continuous
  residence. Residential and logistical mobility form a spectrum.
- [HRAF, Hunter-Gatherers (2025)](https://hraf.yale.edu/ehc/summaries/hunter-gatherers):
  sedentary and seasonally sedentary fishing societies demonstrate that farming is
  not a prerequisite for long-lived settlements.
- [Robinson (2007), Hunter-fisher-gatherer territoriality](https://intarch.ac.uk/journal/issue22/4/2_1.html):
  inherited procurement locations and seasonal riverside/forest residence illustrate
  persistent places embedded in flexible annual rounds.

The preferred direction is **both a representation refactor and evidence-led
movement/ecology balancing**, rather than a blanket limit on seasonal residence.
