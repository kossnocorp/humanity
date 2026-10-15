# humanity-flora

Owns terrestrial plant stocks, community profiles, growth, forage production and
degradation/recovery. It depends only on serialization—not on fauna, humans, world
topology or rendering.

- `stocks.rs`: persistent `Vegetation` and the three-channel `Forage` budget.
- `rules.rs`: grassland/forest/savanna/taiga/tundra/desert/alpine community profiles
  and inspectable `FLORA` assumptions.
- `growth.rs`: climate-dependent production, live biomass growth, stress/recovery
  and harvest-stock turnover.
- `humanity-world/src/flora.rs`: translates world biomes/weather into inputs.

Plant state is serialized under `cells[].resources.vegetation`. Nutrition composes
that state with aquatic food, derived small-game biomass and toolstone; convenient
Rust stock access delegates through `Resources` to `Vegetation`.

## Forage limits

Forest understory is not another full grassland: community profiles allocate grass
and woody cover separately. Grazing/browse are living forage biomass. Their
logistic growth is limited by standing biomass and climate. The maximum productive
flow is `r*K/4`; only the configured share is available to modeled wildlife.
Fruit is reproductive output of the underlying plants, not a self-replicating pile
of fruit. Edible roots and collected materials retain their regional turnover model.

The wildlife budget is averaged over one season and bounded by standing reserves
covering 90 days. This lets dormant vegetation support overwintering animals without
pretending that winter generates fresh grass. Exhausted or unproductive patches
do not receive an arbitrary positive animal-capacity floor.

**Every species shares the same local budget.** Fauna allocates grass, browse and
fruit among all resident consumers according to their diets and daily food needs.
Its species-density limits are additional ceilings, not fourteen independent
allowances to consume the same pasture. Incoming dispersal groups are included in
the destination's projected demand before admission. Unsupported growth is suppressed;
existing animals are not instantly deleted to meet a quota.

## Overgrazing and recovery

Chronically low grass/browse biomass reduces plant vigor and therefore future
cover, biomass capacity and production. Unused patches recover gradually. A small
root/seed reserve permits regrowth when conditions improve; it is not edible food
or an animal respawn mechanism. Picking fruit alone does not destroy its parent
plant. Dry/frozen conditions limit new production rather than immediately erasing
all standing winter forage.

These are plant **guilds**, not individual botanical species or a complete carbon,
soil, hydrology or succession model. Units and coefficients are prototype tuning
assumptions, not an empirical carrying-capacity estimate. Tests cover shared food
budgets, absent-pasture limits, understory differences, stress/rest and dormancy.

The viewer shows cover, vigor, seasonal forage supply and wildlife demand. Large
game retains herd markers; small colonies use a rabbit-shaped marker only at close
zoom or in the Food layer. This visual distinction is separate from the actual
population constraints. See `docs/research/flora.md` for the before/after audit.
