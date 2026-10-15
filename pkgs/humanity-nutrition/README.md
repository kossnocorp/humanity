# humanity-nutrition

World-independent food data and mechanics:

- `foraging.rs`: density-dependent plant/fruit search and stock subtraction.
- `hunting.rs`: capture success from participating hunters' skills.
- `humanity-fauna`: owns herd/species data, encounter, actual trapping/harvest,
  animal diets, reproduction, avoidance and conservative dispersal.
- `fishing.rs`: patch encounter/travel/weather returns and actual catch subtraction.
- `consumption.rs`: age-sensitive rations, eating, preservation/spoilage and personal
  health response to the fraction of real food need met.
- `stocks.rs`: `Resources` composes `humanity-flora::Vegetation`, `AquaticLife`,
  toolstone, harvest-driven local depletion and derived small-game biomass.
  `game` is a view of living fauna, not renewable meat. Plant stocks/growth are owned
  by `humanity-flora`, with persisted vegetation nested in resource snapshots.
- `rules.rs`: inspectable `FORAGING` assumptions formerly misplaced in cultures.

Functions receive environmental/work inputs rather than importing `World`.
Simulation orchestrates resource selection, workers, RNG rolls and events; world
owns seasonal habitat renewal and `src/climate.rs` hydrological rules. This avoids
cyclic dependencies while keeping the actual acquisition math inspectable here.

Food remains a shared band pool measured in adult person-days. Child rations vary
with age; infancy's allowance includes a coarse nursing/provisioning cost. This is
not a complete maternal energy or micronutrient model. Depletion reduces accessible
returns without additionally deleting remote biomass. Local patches recover with
rest, and relocation spends working time. Coefficients are prototype ecological
assumptions, not fitted mortality percentages.
