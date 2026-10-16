# Biome variety (v11)

Generation now combines spatial noise with a wet equatorial and dry subtropical
moisture pattern. Temperature, elevation and moisture select vegetation; sufficiently
wet, river-fed lowlands below 450 m can become marsh or warm wooded swamp.
Wetlands remain traversable land rather than boat-only water cells. Four new biomes
join the existing twelve; no requirement forces all sixteen into every world.

`humanity-world::classify_biome` contains the climate envelopes. Plant community
cover lives in `humanity-flora`; biome fertility and travel costs live in core.
Wetlands gain rainfall-dependent recharge and freshwater aquatic capacity. Foot
travel costs use the mean of source and destination terrain; existing navigable
river routes retain their boat advantage. Travel credit can accumulate sufficiently
to escape the slowest terrain. Woodland concealment includes swamps consistently
in human hunting, fauna and rendering.

Camels prefer arid land, tapirs tropical forest/swamp, and water buffalo freshwater
habitat. These are prototype ecological archetypes, not a reconstruction of any
single real continent. Their diets, food needs and life-history coefficients are
inspectable in `humanity-fauna/src/species.rs`. Habitat suitability never creates
food: specialist animals still compete for the same production/reserve budgets.
Sparse dry vegetation makes camel persistence especially fragile in these runs.

Challenges use resource scarcity, cover, cold/snow, hydrology and travel costs.
There is no newly implemented thirst meter, infectious disease, predator system or
seasonal flood inundation. Biome labels are fixed after generation; weather and
vegetation remain dynamic. Moisture belts and wetland elevation thresholds are
explicit prototype assumptions rather than a full atmospheric/floodplain model.

## Saved-state checks

Resolution 28, 20 km cells, temperature 28, 90-day seasons, 6,000 initial humans:

| Seed / rain / elapsed years | Desert | Rainforest | Marsh / swamp | Camel / tapir / buffalo | Humans |
|---|---:|---:|---:|---:|---:|
| 42 / 1.0 / 14 | 176 | 365 | 9 / 18 | 9 / 65 / 124 | 2,925 |
| 7 / 0.7 / 20 | 423 | 16 | 1 / 0 | 5 / 0 / 15 | 1,167 |
| 123 / 1.3 / 20 | 117 | 403 | 20 / 20 | 2 / 188 / 59 | 3,288 |

Seed 42 also contains 180 steppe and 111 scrubland cells. All 17 species remain
at year 14, totaling 274,941 animals. Species-by-species initial + births − deaths
− harvest accounting was checked in all three saved states. The dry world's lack
of surviving tapirs and swamps is allowed, not repaired by respawning.

Tests cover climate/drainage classification, generated biome variety, wetland fish
and traversal, and specialist preferences without overriding absent food. Fishing
fixtures explicitly initialize individual skills and stock density, avoiding
dependence on incidental generated people or competing wildlife jobs.
