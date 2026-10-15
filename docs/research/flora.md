# Flora and shared carrying capacity (v10)

The v9 fauna rework restored recolonization but allowed excessive population/spatial
growth. This follow-up introduces `humanity-flora` and couples all consumers to one
local production budget, without a global animal-count or occupied-cell quota.

## Diagnosis

1. Each species received its own `density × area × suitability × biomass` ceiling.
   Species competing for the same grass could each claim the pasture independently.
2. The previous capacity used standing biomass rather than sustainable production
   and included a positive 2% floor even when food was absent.
3. Forest cells produced approximately as much grazing as open grasslands while
   also producing woody browse. This duplicated productive habitat.
4. Grazer fallback feeding could consume fruit even though its configured dietary
   fruit weight was zero.
5. Dispersal checked current same-species occupancy, not the food demand of the
   entire community plus the incoming animals.
6. Every small rabbit/bird colony used the same map icon as large game. Marker
   coverage overstated the spatial extent of actual large herds.

## Implementation

- Flora owns edible plants, fruit, grass, browse, wood/fiber, vigor, community cover
  and sustainable forage supply. World supplies weather/geography; fauna consumes
  the resulting real stocks. State remains inspectable and deterministic.
- Grass and browse use logistic biomass growth. Their intrinsic rates are scaled
  so `r*K/4` retains the previous maximum-renewal order of magnitude; simply reusing
  a stock-relaxation coefficient as a logistic rate would accidentally quarter it.
- Structural pasture/browse capacity is separate from seasonal production. An early
  trial that reduced both capacity and growth with cold erased winter reserves and
  caused excessive losses; this was rejected. Fruit/seed output follows plant vigor
  and season rather than treating picked fruit as destruction of its parent plant.
- Half the maximum productive flow is budgeted to modeled fauna, averaged across a
  season and capped by standing food reserves for 90 days. These are explicit
  prototype utilization/planning assumptions, not measured paleoecological rates.
- Resource shares account for every species' preferred diet and daily food needs.
  Combined allocated carrying demand cannot exceed the shared forage budget.
  Recruitment slows when the community oversubscribes it. Dispersal projects the
  incoming group's needs into the destination before allowing a transfer.
- Persistent overgrazing reduces vigor/cover and future production; resting ground
  recovers gradually. No arbitrary herd purge, respawn or minimum animal population
  is introduced. Species can still become extinct.

## Matched-configuration checks

Seed 42, resolution 28, 20 km cells, temperature 28, 90-day seasons, initial human
population 6,000; 2,741 land cells. Counts below describe actual population state,
independently of marker visibility.

| Day 5,040 / year 14 | v9 | v10 |
|---|---:|---:|
| Living animals | 1,512,377 | 312,528 |
| Wildlife population records | 6,509 | 2,326 |
| Cells with any wildlife | 2,088 (76.2% of land) | 1,580 (57.6%) |
| Cells with large game | 1,102 (40.2% of land) | 265 (9.7%) |

This is about 79% fewer animals and 76% fewer large-game cells at that point, not
merely hidden icons. All 14 species were still present in the v10 year-14 snapshot.
Small wildlife can legitimately remain widespread in suitable habitat; its new
marker appears only on close inspection or the Food layer, rather than labelling
every colony as a large herd on the default map.

Longer v10 checks:

- Seed 42, year 100: large game in 150 cells; deer 824, boar 1,049, reindeer 115,
  bison 40, antelope 97, ibex 100, elk 59 and wild horses 41. Mammoths became extinct;
  the other 13 species remained. Human population was 3,293.
- Seed 7, year 50: all 14 species remained, including 13 mammoths; large game in
  210 cells. Human population was 1,518.
- Seed 123, year 50: all 14 species remained, with one mammoth left; large game in
  149 cells. Human population was 2,560.

These checks demonstrate limitation and persistence under several scenarios, not
empirical calibration or universal survival. Large herbivores are now much rarer
than small fauna. Predators, detailed plant species, soil succession and finer
foraging ranges remain future work; no claim is made that the present numeric
densities reproduce a particular real ecosystem.

Tests separately cover no seeding/growth without productive forage, cross-species
competition for a single budget, no fruit-only subsistence for grazers, forest
understory versus grassland, overgrazing damage/recovery, and winter reserves.
Existing animal conservation/extinction and deterministic resume checks still apply.
Fresh v10 saves are required; old formats are not migrated.

The century snapshot passed per-species population-budget and flora-state checks.
A further 180-day run was byte-identical to 90 days + save/load + 90 days, including
biomass, vigor and the smoothed supply history. The 14-year snapshot was rendered
to check the sparser large-game markers and the supply/demand field notes.

```sh
cargo run -p humanity-cli --no-default-features -- --headless --resolution 28 \
  --temperature 28 --season-days 90 --population 6000 --seed 42 --days 5040 \
  --report-every 360 --save saves/flora-14y.json
```
