# Cultural rulebook

**Edit [`src/lib.rs`](src/lib.rs) to inspect or tune the current cultural model.**
This crate is renderer-independent and contains the complete starting variant registry and residential/social parameters. These are fictional strategies, not historical ethnicities or a technological sequence.

## Starting variants (`VARIANTS`)

| Variant | Residential custom | Locations / movement | Inland : waterside weights | Initial fishing / preservation floors |
|---|---|---|---|---|
| `DispersedForagers` | Small-camp dispersal | Mobile | 35 : 10 | 0 / 0 |
| `CooperativeForagers` | Balanced fission | Mobile | 30 : 15 | 0 / 0 |
| `RiverNetworks` | River budding | Mobile | 5 : 25 | .30 / .20 |
| `FishingResidents` | River budding | Persistent fishing settlements | 0 : 35 | .50 / .40 |
| `SeasonalRounds` | Balanced fission | Remembered, season-specific camps | 30 : 15 | 0 / .30 |

Weights are relative within inland or waterside environments. Initial culture identities group bands by dominant language family and variant. Each identity has a stable ID, name, variant, founding day and up to two parent identities. Multiple identities can use the same variant. Bands remain aggregate age/sex cohorts, not person entities.

## Camp splitting (`CAMPS`)

| Custom | Normal threshold | Pressure days | Departing party |
|---|---:|---:|---|
| Dispersed | 40 | 14 | Half the current threshold, at most half the people |
| Balanced | 70 | 28 | Half the people |
| River budding | 100 | 45 | Half the threshold, at most one third of the people |

`departing_fraction` and `departing_limit_fraction` tune the allocation. Thresholds count **all ages**, not just adults. Initial bands are about 50–83% of the threshold regardless of world population. Splitting conserves every cohort and all portable possessions, inherits culture/languages/knowledge, and allocates a new stable band ID. Camps begin in the same cell, then move normally. This also allows several camps to occupy one fishing settlement without turning it into a single giant band.

## Ecological adjustments (`SOCIAL`)

- Rich river/fishing camps: threshold 150 with fishing ≥.30, preservation ≥.25, food ≥8 days/person and accessible aquatic stocks ≥30 days/person.
- Food below 3 days/person or health below .65: threshold ×.75.
- Below threshold, crowding pressure recedes by 2/day. Above twice the threshold, split immediately; otherwise wait for the custom's pressure duration.

## Identity change (`SOCIAL`)

- **Isolation:** 3 simulated years without friendly contact with another same-culture band creates a daughter identity, provided other bands of the old culture still exist. A lone surviving culture does not repeatedly split from itself. Same-culture encounters and band fission reset isolation.
- **Variation:** an isolated daughter has a .35 chance to resample its variant using local environmental weights; otherwise it retains the parent's variant.
- **Mixing:** 60 distinct days of friendly contact with a particular other culture creates/adopts a shared two-parent identity. Multiple encounters in one day count once. Switching contact partner culture restarts accumulation; a year without foreign contact clears the memory. Both sides can join the same new identity. Counters count band adoptions, not unique new identities.
- A mixed identity inherits one parent's variant (50/50 by default). Existing language mixing and skill/material exchange accompany contact; identity change grants no skills or resources. No claims of biological ancestry are implied by cultural parent IDs.

## Locations (`SOCIAL`)

Fishing residents and seasonal-round bands establish a location after **20 consecutive residence days**, with at least 3 food-days/person. Fishing settlements additionally require fishing ≥.30 and accessible aquatic biomass ≥10 food-days/person. They are reusable residence/procurement locations, not simulated buildings or farming towns.

There is **one place per culture/cell**, independently of season and function. Sites record local-season associations, current and longest continuous stay, distinct occupation episodes, total occupied days, present associated people, and up to eight recent seasons of occupied days/person-days. Southern seasons are shifted by half a year. A new seasonal association is learned after 10 occupied days in that season (capped at season length); the founding season is known initially.

Function (fishing location / residential camp) is distinct from observed use. A two-season uninterrupted occupation is described as a long-stay base; a full uninterrupted year as observed year-round residence. Otherwise recent occupied days distinguish occasional, seasonal and multi-season use. Four known season associations alone do not establish permanence. These are transparent display criteria, not universal archaeological boundaries or new settlement technology.

Cultures remember at most 12 places. Fishing sites attract return all year; residential sites during known seasons. Secure bands favor their nearest eligible cultural or founder site (return score 8, on-site loyalty 4), multiplied by **condition**; shortage or poor health overrides attachment. Every journey pays normal spherical travel costs and obeys routing constraints.

### Decay and maintenance (`SOCIAL`)

Food security is constrained by `FORAGING` in `humanity-nutrition/src/rules.rs`
and `CLIMATE` in `humanity-world/src/climate.rs`:
stock-density search, a shared near-camp depletion index (4,000 harvested person-days
per standard cell adds one unit), recovery over half a season, distance-costed
single-patch fishing, and half working time during relocation. Seeded interannual
rainfall changes soil moisture, recharge and freshwater capacity. These parameters
are ecological modelling assumptions, not fitted death-cause percentages. See
`../../docs/research/deaths.md` for research and long-run comparisons.

| Parameter | Default | Effect |
|---|---:|---|
| `residential_site_decay_years` | 3 | Years to lose one condition unit without maintenance |
| `fishing_site_decay_years` | 6 | Slower loss for fishing places |
| `site_maintenance_repair` | .025 | Condition repaired per maintained day, after daily decay |
| `site_abandon_condition` | .02 | Remove place at/below this condition |
| `site_season_use_days` | 10 | Meaningful seasonal use / learning threshold |

Associated residents are owning-culture bands and the founder even if it changes culture; a founder is never double-counted. At least one associated band needs health ≥.65 and food ≥3 days/person to maintain the camp. Upkeep represents routine clearing/repair and maintained knowledge, without an extra materials debit or building system. Unsupported visits still record real occupancy but do not reset decay. Normal annual absences leave time for return and repair; long neglect removes the marker even with living owners. Total loss of founder and owning culture also removes it immediately. Removal emits `SiteAbandoned`, frees a site slot, and re-founding allocates a fresh ID/history. Extinct culture records stay in the ancestry archive.

These decay rates are modelling assumptions. A disappearing marker represents a forgotten/unusable active camp, not eradication of all archaeological traces. Sites do not create food, reserve stocks, exclude outsiders or merge bands.

## Inspect / tune / run

1. Edit the typed tables and `SOCIAL` constant in `src/lib.rs`; rebuild with normal Cargo commands. Keep thresholds ≥2, departure fractions between 0 and 1, durations positive, and at least one nonzero seeding weight in each environment.
2. Generation weights and initial proficiency floors affect **new worlds**; existing identities and band customs remain persisted. Runtime thresholds and social rates affect resumed worlds too. Changing rules intentionally changes future trajectories.
3. `cargo run -p humanity-cli --no-default-features -- --inspect saves/camps-10800.json` lists **actual living cultures**, variants, parents, population/bands, remembered locations and size statistics. JSON saves include the complete extinct-identity archive as `cultures`. The camp rework uses v5 saves; older saves require a new world, with no compatibility path.
4. In the viewer, click a cell and scroll field notes for culture/custom/pressure, condition and season-by-season occupation details. Cyan houses mark fishing locations; violet tents mark residential camps. The browser shows their observed occupation pattern independently of function.

Implementation consumers: `humanity-world` seeds cultures; `humanity-sim/src/social.rs` handles fission, divergence, mixing and location lifetime; movement is in `humanity-sim/src/lib.rs`; persisted records are in `humanity-core`. Practical-skill learning remains in `humanity-sim/src/subsistence.rs`; this crate currently centralizes residential/social rules, not every ecological or demographic rate.

Research and limitations: [`PREHISTORY.md`](../../PREHISTORY.md).
