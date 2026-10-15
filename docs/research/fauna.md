# Fauna persistence, hunting and extinction

This describes the v9 rework. The subsequent [v10 flora/carrying-capacity audit](flora.md)
corrects overexpansion from per-species pasture allowances and documents newer results.

2026-10-15. Goal: sustained wildlife and hunting in viable ecosystems, with local
depletion and genuine possible extinction, rather than either guaranteed survival
or automatic disappearance of nearly every species.

## Evidence guiding the mechanisms

- **Gaynor et al. (2018), “The influence of human disturbance on wildlife
  nocturnality.”** [Science / DOI](https://doi.org/10.1126/science.aar7121),
  [open manuscript](https://escholarship.org/content/qt87b547gj/qt87b547gj_noSplash_e8c83e9140044f66cf1f2d9e4d2bb519.pdf).
  Meta-analysis of 76 studies covering 62 mammal species reports greater
  nocturnality under disturbance (average ratio 1.36). Supports reduced encounter
  and temporal/spatial avoidance; it does not imply that all species react alike,
  that the numerical effect is a prehistoric constant, or that animals vanish.
- **“Measuring local depletion of terrestrial game vertebrates by central-place
  hunters in rural Amazonia” (2017).** [Full text / DOI](https://doi.org/10.1371/journal.pone.0186653).
  Camera traps and interviews across 60 communities show species-dependent
  depletion/detectability, including greater sensitivity of large/group-living
  game. Remote source areas can replenish hunted sinks. These relatively low-
  density communities with aquatic alternatives had not universally emptied
  surrounding forests. This is a conditional coexistence example, not proof that
  all subsistence hunting is sustainable.
- **“Mammal responses to global changes in human activity vary by trophic group
  and landscape” (2024).**
  [Nature Ecology & Evolution](https://www.nature.com/articles/s41559-024-02363-2).
  Cross-project evidence emphasizes species traits, habitat modification and
  different responses to people. Useful for keeping generalists, shy grazers and
  habitat specialists distinct instead of one generic animal behavior.

These contemporary studies inform mechanisms, not a Paleolithic species census or
target mortality/harvest percentages. No claim is made that modern Earth's animal
densities can be copied directly into a procedural prehistoric planet.

## Previous implementation: why game faded

1. All six species consumed only the same grazing stock, including woodland deer
   and boar. No woody browse or omnivore food alternatives existed.
2. Breeding dropped to .03/year outside a narrow temperature interval, while even
   healthy animals died at .09/year. Nutritional condition also declined very
   rapidly, with the same starvation response for all species.
3. Initial groups could move/reproduce but never establish daughter populations.
   Extirpated regions could only be revisited by entire surviving herds; the number
   and geographical coverage of populations tended to collapse.
4. Hunting selected the first nearby group, retained a capture floor independent
   of prey density, and did not represent rarity, concealment or learned wariness.
   Searching for the last few animals was almost as easy as finding an abundant herd.
5. An independent renewable `game` food pool hid untracked small fauna; its harvest
   was not counted as animals hunted. This is now a derived view of actual animals.

Fresh v8 baseline, seed 42, resolution 28, temperature 28, 20 km cells, 90-day
seasons, 6,000 initial humans:

| Year | Remaining groups | Remaining animals by original species |
|---:|---:|---|
| 10 | 65 | Antelope 1,458; bison 3,735; boar 172; deer 362; reindeer 34; ibex 0 |
| 30 | 36 | Antelope 1,442; bison 3,541; boar 37; deer 1,486; reindeer/ibex 0 |
| 100 | 17 | Antelope 143; bison 2,751; deer 49; boar/reindeer/ibex 0 |

Tracked harvest in years 90–100: 2,877 animals, from only three surviving species
and very few remaining locations. This seed did not lose literally all animals,
but much of the world lost access to large game.

## Implemented model

`humanity-fauna` owns 14 species, habitat/diet traits, population state, recruitment,
mortality, avoidance, dispersal, encounter and harvest accounting. Nutrition owns
capture skill/food calculations; world supplies habitat and copies consumed stocks
back. Group animals remain aggregate; human identities are unchanged by this extraction.

- Browsers, grazers and omnivores consume different renewable resources.
- Recruitment is resource/density limited, seasonal and condition-dependent;
  thermal suitability is smooth. Effective recruitment includes age/sex structure.
- Disturbed populations become wary and may seek quieter habitat. Concealment,
  distance and scarcity reduce encounters; no artificial minimum population is protected.
- Surviving groups can split and disperse, conserving animals. Colonization depends
  on land connectivity, travel and habitat; waterfowl have limited water crossings.
- Actual small animal populations replace the independently replenished game pool.
- Statistics distinguish recruitment, natural deaths, harvest and group transfers.
  Save validation checks each species' complete animal budget and small-game biomass.

All coefficients are inspectable prototype assumptions. Mammoths are intentionally
slower to recover than rabbits or boar, but tests also require slow breeders to
persist/spread in suitable unhunted refuges. A fractional breeder penalty was
removed during development because recruitment rates already include nonbreeders:
double-counting it made small daughter groups structurally nonviable. The remaining
minimum pair requirement permits isolated-last-animal extinction without resurrection.

Missing mechanisms include explicit predators, detailed animal age/sex structure,
regional disease and full long-distance migration. Wariness approximates behavioral
avoidance rather than simulating a day/night schedule. These boundaries matter when
interpreting survival and harvest counts.

## Verification approach

Mechanism tests cover conservative dispersal/merging, depleted-area recolonization,
resource-limited feeding, viable browser/generalist diets, human displacement,
reduced detection of rare/wary animals, survival of slow breeders in good refuges,
and genuine extinction/no respawn. Integration checks reconcile real harvested
animals and consumed plant resources rather than treating meat as free renewal.

Headless evaluations use multiple seeds, century-scale continuation and a dense
small-planet stress scenario. Compare original species separately from added small
fauna: total harvested animals is not directly comparable to the old counter, which
excluded the renewable small-game pool. Fresh worlds share configuration/geometry
for a seed, but initial human placement changes because expanded fauna generation
consumes a different number of RNG draws. These are scenario comparisons, not a
strict single-mechanism causal ablation or empirical validation.

### Measured v9 results

All ordinary runs use temperature 28, 20 km cells and 90-day seasons. Each retained
all 14 initially present species at its endpoint; that observation is not a
guarantee for other seeds, climates, densities or longer horizons.

| Seed / resolution / initial humans | Years | Living animals | Wildlife groups | Living humans | Original-six harvest in final decade | All-species harvest in final decade |
|---|---:|---:|---:|---:|---:|---:|
| 42 / 28 / 6,000 | 100 | 1,324,187 | 7,589 | 2,426 | 9,263 | 57,077 |
| 7 / 28 / 6,000 | 50 | 1,479,304 | 7,518 | 3,054 | 7,617 | 48,226 |
| 123 / 28 / 6,000 | 50 | 1,265,866 | 6,553 | 2,806 | 6,467 | 44,395 |
| 42 / 6 / 12,000 (dense small planet) | 20 | 50,988 | 208 | 734 | 210 | 2,464 |

The original six are reindeer, bison, antelope, deer, boar and ibex. The old seed-42
world harvested 2,877 of these in years 90–100; the new world harvested 9,263 over
that interval. Wildlife now remains distributed across many viable locations and
hunting continues after skill maturation. Much of the larger total animal count
is small fauna (hares/rabbits/birds), not a million large ungulates.

Recovery is not universal. In the seed-42 century run, mammoths declined from 115
to 11, with 132 births, 234 natural deaths and 2 harvested. Their rarity reflects
slow recruitment, habitat/capacity and stochastic losses, not solely hunting.
In the dense small-planet case they instead grew from 6 to 9 over twenty years.
Mechanism tests additionally demonstrate complete extinction and no resurrection,
and persistence/spread of slow breeders in favorable unhunted habitat.

Human food balance also changes: the independently regenerated small-game food pool is gone,
wildlife consumes fruit, and actual animals can avoid hunters. Human populations
fell from their initial distributions in these runs; this is not a claim that the
combined demographic/ecological model is fully calibrated. Retaining wildlife is
not sufficient evidence of realistic human carrying capacity. Further calibration
is listed in `TODO.md`, rather than concealed behind guaranteed food or survival.

The century snapshot reconciles every species' live population against initial
animals + recruits - natural deaths - harvest. A 180-day uninterrupted continuation
was byte-identical to 90 days + save/load + 90 days, including group IDs, wariness,
dispersal, stocks and gameplay RNG. The viewer was exercised on a century-old cell
with seven wildlife species; markers are deduplicated per occupied cell.

Reproduction:

```sh
cargo run -p humanity-cli --no-default-features -- --headless --resolution 28 \
  --temperature 28 --season-days 90 --population 6000 --seed 42 --days 36000 \
  --report-every 3600 --save saves/fauna-century.json
cargo run -p humanity-cli --no-default-features -- --inspect saves/fauna-century.json
```

Snapshots/logs/screenshots used for the evaluation are under the harness temporary
directory; no existing user save was overwritten. Fresh v9 worlds are required.
