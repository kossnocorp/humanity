# humanity-nutrition

World-independent food data and mechanics:

- `foraging.rs`: density-dependent plant/fruit search and stock subtraction.
- `hunting.rs`: trapping and large-game success from participating hunters' skills.
- `hunting/species.rs`: herd records, prey species and biological/harvest parameters.
- `fishing.rs`: patch encounter/travel/weather returns and actual catch subtraction.
- `consumption.rs`: age-sensitive rations, eating, preservation/spoilage and personal
  health response to the fraction of real food need met.
- `stocks.rs`: `Resources`, `AquaticLife`, harvest-driven local depletion.
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
