# Game model and follow-up work

## Current core entities

| Entity | Meaning |
|---|---|
| World | Planet configuration, spherical grid, time, deterministic RNG, entities and history |
| Cell | Elevation, biome, drainage/river, weather and resource stocks |
| Band (`Tribe`) | Aggregate residential group with population, specialists, food, health, equipment, knowledge, language shares and culture membership |
| Demography | Female/male children, adults and elders; no individual-person entities |
| Specialists | Hunters, gatherers, healers and guides allocated from adults |
| Culture | Named shared identity with variant, founding date and up to two parent identities |
| Cultural site | Remembered fishing settlement or seasonal camp, ownership, founder and visit history |
| Herd / species | Moving aggregate animal population, condition and travel; six land species |
| Language mixture | Family shares within a band; overlap affects mutual understanding |
| Knowledge | Cohort proficiency in 13 practical skills and species-specific hunting familiarity |
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
`BandFission`, `CultureDivergence`, `CultureMixing`, `SiteFounded`.

Events record outcomes; simulation systems execute mechanics. Each event includes day,
cell, band, optional other participant, amount and probability field. Recent history
retains 512 events; lifetime totals and population diagnostics are separate.

### Code map

- `pkgs/humanity-core/src/lib.rs`: world, cells, bands, demography, herds, events.
- `pkgs/humanity-core/src/culture.rs`: cultural identities/sites, knowledge/equipment.
- `pkgs/humanity-core/src/insights.rs`: population/mortality records.
- `pkgs/humanity-sim/src/lib.rs`: daily phases, movement, feeding, contacts, demography.
- `pkgs/humanity-sim/src/subsistence.rs`: crafting, fishing and practical learning.
- `pkgs/humanity-sim/src/social.rs`: identity change, camp fission and remembered sites.
- `pkgs/humanity-world/src/lib.rs`: generation, climate and ecological renewal.

## Seasonal locations: investigation (2026-10-12)

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

### Causes in current implementation

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

### Recommended refactor (pending)

- [ ] Give a remembered place one stable identity; represent multiple seasons of
  use on that place instead of creating a place per season. Initially deduplicate
  per culture/cell; consider shared physical places with separate cultural associations
  so ancestral ownership and founder access remain explicit.
- [ ] Separate **function** (residential base, fishing location, resource/task camp)
  from **observed occupation pattern** (seasonal, multi-season, near-year-round).
  Culture should bias mobility, not permanently dictate a site's classification.
- [ ] Track season-specific occupied days, distinct occupation episodes,
  consecutive occupancy, absences, residents and recent person-days. Keep historical
  use separate from recent use so centuries of visits do not imply current permanence.
- [ ] Preserve legacy season flags as recorded associations, but mark historical
  occupancy unknown: `founded`/`last_visited` cannot reconstruct past residence.
- [ ] Base descriptive labels on observed duration/repetition, with minimum evidence
  and hysteresis. Four visited seasons alone must not imply continuous settlement.
- [ ] Measure and balance expected **net foraging returns**, localized depletion,
  renewal, accessible daily range, water, storage and seasonal resource pulses.
  Distinguish short foraging trips from moving the whole residential camp.
- [ ] Revisit population-based crowding, self-counting and switching costs to reduce
  artificial A→B→A moves. Test attachment strengths separately from ecological renewal.
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
