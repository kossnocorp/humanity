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

Sites remember cell, owning culture, founding band, founding/last-visit dates, and (for seasonal camps) local season. Southern seasons are shifted by half a year. Cultures remember at most 12 sites; fishing sites are year-round, seasonal sites attract return during their recorded season. Secure bands favor their nearest eligible cultural site (return score 8, on-site loyalty 4); shortage or poor health overrides that attachment. Every journey still pays spherical travel costs and obeys land/boat routing.

A marker remains while **either its founding band or its owning culture has living members**, even when unoccupied. It disappears when both are extinct. Extinct culture records stay in the ancestry archive. Sites do not create food, reserve stocks, exclude outsiders, merge resident bands, or require residents to remain through famine.

## Inspect / tune / run

1. Edit the typed tables and `SOCIAL` constant in `src/lib.rs`; rebuild with normal Cargo commands. Keep thresholds ≥2, departure fractions between 0 and 1, durations positive, and at least one nonzero seeding weight in each environment.
2. Generation weights and initial proficiency floors affect **new worlds**; existing identities and band customs remain persisted. Runtime thresholds and social rates affect resumed worlds too. Changing rules intentionally changes future trajectories.
3. `cargo run -p humanity-cli --no-default-features -- --inspect saves/residential-3600.json` lists **actual living cultures**, variants, parents, population/bands, remembered locations and size statistics. JSON saves include the complete extinct-identity archive as `cultures`.
4. In the viewer, click a cell and scroll field notes for culture/custom/pressure and location details. Cyan house markers are fishing settlements; violet tents are seasonal locations.

Implementation consumers: `humanity-world` seeds cultures; `humanity-sim/src/social.rs` handles fission, divergence, mixing and location lifetime; movement is in `humanity-sim/src/lib.rs`; persisted records are in `humanity-core`. Practical-skill learning remains in `humanity-sim/src/subsistence.rs`; this crate currently centralizes residential/social rules, not every ecological or demographic rate.

Research and limitations: [`PREHISTORY.md`](../../PREHISTORY.md).
