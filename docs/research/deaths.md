# Prehistoric mortality and the simulation

Follow-up: [population replacement and extinction audit](population-balance.md)
examines the later collapse without changing simulation parameters. The cause
shares below are measured outcomes, **not scientifically validated targets**.

Research checked 2026-10-14. **There is no defensible universal Paleolithic or
Neolithic percentage distribution of deaths by cause.** Skeletal stress markers,
survived injuries, pathogen evidence, and ethnographic mortality records answer
different questions. In particular, low *direct starvation* mortality is not by
itself historically implausible; a world in which almost everybody permanently
has perfect health and effortless food surpluses needs a mechanistic audit.

## Paleolithic

- **Houldcroft (2023), “Infectious disease in the Pleistocene: Old friends or old
  foes?”** [DOI](https://doi.org/10.1002/ajpa.24737),
  [PubMed](https://pubmed.ncbi.nlm.nih.gov/38006200/),
  [open manuscript](https://api.repository.cam.ac.uk/server/api/core/bitstreams/9fb845bf-2551-41db-a23f-eec23191a5b3/content).
  Reviews ancient DNA and other evidence for infection in humans and hominins.
  Small mobile populations were not free from pathogens: zoonoses, parasites and
  contaminated food/water matter, alongside demographic limits on transmission.
  This is evidence for disease mechanisms, not a census of causes of death.
- **Gurven & Kaplan (2007), “Longevity Among Hunter-Gatherers: A Cross-Cultural
  Examination.”** [Paper](https://gurven.anth.ucsb.edu/sites/secure.lsit.ucsb.edu.anth.d7_gurven/files/sitefiles/papers/GurvenKaplan2007pdr.pdf),
  [bibliographic record](https://ideas.repec.org/a/bla/popdev/v33y2007i2p321-365.html).
  Comparative *recent ethnographic* data, not Paleolithic observations. The
  reported pooled causes are approximately 70% illness, 20% violence/accidents,
  9% degenerative disease. Do not relabel the illness share as starvation or use
  these pooled proportions as prehistoric targets. Contact histories, reporting,
  age structures, and local ecology complicate comparisons.
- **Hill et al. (2007), “High adult mortality among Hiwi hunter-gatherers.”**
  [Paper](https://www.unm.edu/~robwal/hiwi.pdf). Useful counterpoint to pooled
  averages: markedly different mortality and trauma patterns among foragers.
- **Spikins et al. (2019), “Living to fight another day.”**
  [Open manuscript](https://eprints.whiterose.ac.uk/id/eprint/134559/1/Significance_of_Neanderthal_healthcare_revisedversion.pdf).
  Neanderthal injury, survival, care, and nutritional-stress evidence. Healing
  and enamel hypoplasia indicate lived experience, not necessarily fatal causes.
- **Venkataraman et al. (2017), hunter-gatherer residential mobility and marginal
  value of rainforest patches.** [DOI](https://doi.org/10.1073/pnas.1617542114).
  Contemporary foraging evidence for declining acquisition rates and mobility
  before complete exhaustion. Relevant to modelling search costs and local
  depletion, not to assigning a death percentage.

**Implications:** allow recurrent shortages through seasonal availability,
resource density, travel/work budgets, depleted patches and correlated bad years.
Knowledge, sharing, storage and migration should buffer these risks rather than
eliminate ecological constraints. Infection, accidents, maternal/infant risks and
age-specific vulnerability remain important missing mechanisms. The game's
“Nutrition / poor health” category currently includes health-mediated risk after
conflict or travel; it is not a diagnosis of starvation.

## Neolithic

- **Milner (2019), “Early agriculture's toll on human health.”**
  [Full text](https://pmc.ncbi.nlm.nih.gov/articles/PMC6628670/),
  [DOI](https://doi.org/10.1073/pnas.1908960116). Agriculture raised caloric yield
  per area but narrower diets and failed harvests could increase vulnerability;
  crowding, livestock and contaminated food/water increased pathogen exposure.
  Explicitly cautions against a universal stepwise decline in health: conditions
  changed within communities, and morbidity is not identical to mortality.
- **Larsen et al. (2019), “Bioarchaeology of Neolithic Çatalhöyük reveals
  fundamental transitions in health, mobility, and lifestyle in early farmers.”**
  [DOI](https://doi.org/10.1073/pnas.1904345116). Primary study discussed by Milner:
  diet, infection indicators, dental development, activity and cranial trauma in
  a particular settlement. Not a general Neolithic cause-of-death table.
- **Wells & Stock (2020), “Life History Transitions at the Origins of Agriculture:
  A Model for Understanding How Niche Construction Impacts Human Growth,
  Demography and Health.”** [Full text](https://pmc.ncbi.nlm.nih.gov/articles/PMC7253633/).
  Useful framework linking energy allocation, growth, fertility, disease and
  changing subsistence instead of treating food production as health alone.
- **Wood et al. (1992), “The osteological paradox: Problems of inferring
  prehistoric health from skeletal samples.”**
  [DOI](https://doi.org/10.1086/204084). Selective mortality, heterogeneous frailty
  and demographic nonstationarity complicate inference. A skeleton with lesions
  may represent survival long enough for lesions to form; acute fatal illness may
  leave none. Applies equally to Paleolithic interpretation.
- **DeWitte & Stojanowski (2015), “The osteological paradox 20 years later.”**
  [DOI](https://doi.org/10.1007/s10814-015-9084-1). Further methodological context;
  useful before translating skeletal lesion prevalence into simulation mortality.

**Implications:** future farming should change diet breadth, storage dependence,
labor and infection ecology together. Do not simply add a Neolithic death-rate
multiplier. Neither period supports enforcing a minimum nutrition-death share.

## Reproducing the current anomaly

Fresh v6 world: seed 42, resolution 28, 20 km cells, temperature 28, 90-day seasons,
6,000 initial people; run 10,800 days (30 years). Daily persisted diagnostics:

| Years | Nutrition/poor-health deaths | Share of deaths | Mean health |
|---|---:|---:|---:|
| 0–5 | 364 | 24.76% | .99798 |
| 5–10 | 47 | 3.91% | .99943 |
| 10–20 | 36 | 1.58% | .99960 |
| 20–30 | 1 | .05% | .99971 |

At year 30, median food reserves were 44.56 person-days/person. In years 20–30,
only about 0.00003% of population-days had less than one day's stored food.
This is an ecological/production saturation problem, not a chart-window bug.

### Diagnosis and implemented changes

1. Skills approach high proficiency within a few years. Old fishing capacity could
   exceed household daily need several times over. The catch loop retained its
   unused budget after unsuccessful attempts and swept up to seven cells. There
   was no distance/search-time debit. Now a band chooses one best aquatic patch,
   pays distance and encounter costs, and gives time back to land foraging when
   expected aquatic returns are poor.
2. Gathering/trapping previously produced full returns until stocks were almost
   empty. Now encounter rates vary continuously with stock density; fruit/plant
   working time follows availability, without refunding unsuccessful search.
3. A 20 km cell's biomass was effectively all equally reachable. A persisted
   `foraging_pressure` now represents depletion of the accessible subpatches:
   harvested food adds pressure, rest restores accessibility. It reduces search
   yields rather than removing additional food. Stock subtraction still conserves
   actual harvest. The common index is a coarse approximation, not explicit trips
   or separate species-specific local patch maps.
4. Residential moves formerly allowed fishing at the origin then gathering at
   the destination with no loss of working time. Moves now precede subsistence;
   relocation leaves half a working day for acquisition/crafting. Movement counts
   other residents by people rather than bands and no longer counts self as a
   competitor. Remembered-site and food security preferences remain.
5. Fast weather varied but there were no persistent multi-year rainfall anomalies,
   river recharge had an unconditional floor, and freshwater fish ignored water
   availability. Seeded smooth regional wet/dry spells now affect recharge and
   soil moisture, which also constrain freshwater carrying capacity. The two
   climate periods are illustrative assumptions, not measured paleoclimate.

All coefficients live in `humanity-cultures/src/ecology.rs`. No mortality hazard,
death quota, minimum cause share, or arbitrary new death event was added. The
existing cubic poor-health hazard still resolves actual deficits. A first trial
with much stronger search constraints caused excessive early population loss and
was rejected. Search alone still left mature survivors permanently provisioned;
resting/revisited patches and relocation costs address that missing dynamic.

### Validation results (new v7 worlds)

Same seed-42 configuration as above:

| Years | Nutrition/poor-health deaths | Share | Population at end |
|---|---:|---:|---:|
| 0–5 | 1,564 | 57.21% | 4,272 |
| 5–10 | 650 | 41.67% | 3,553 |
| 10–20 | 780 | 38.42% | 2,751 |
| 20–30 | 501 | 36.15% | 2,473 |
| 40–50 | 263 | 29.19% | 1,805 |
| 60–70 | 306 | 40.75% | 1,135 |
| 90–100 | 132 | 30.84% | 1,002 |

Independent seeds, same configuration, years 20–30: seed 7 had 550 nutrition/poor-
health deaths (38.57%, 2,138 survivors); seed 123 had 687 (48.28%, 1,825 survivors).
**These are regression observations, not historical target percentages.** The
initial population and its distribution are not an ecological equilibrium. The
remaining long-run population decline, age-specific vulnerability and missing
infection ecology need demographic calibration; the work establishes recurrent
resource-mediated stress rather than claiming historically validated mortality.

Seed 42's years 20–30 now have 1.37% of population-days below one stored food-day,
versus 0.00003% before; median reserves at year 30 are 20.23 days instead of 44.56.
Most people can still be well provisioned while particular bands experience lean
spells. These diagnostics are more informative than requiring a fixed death share.

Tests cover declining catch before stock exhaustion, skilled fishers still paying
distance/depletion/relocation costs, food-stock conservation, patch recovery,
freshwater drying, RNG independence, and deterministic save/resume. A century-long
headless run retained positive population and reconciled death-cause totals.
The 180-day whole versus 90+90-day resumed continuation was byte-identical after
enabling exact JSON float round-tripping: the check exposed a one-ULP rounding
error in a cumulative food total, now covered by the save/resume regression test.

Reproduction (change seed or `--days` to compare):

```sh
cargo run -p humanity-cli --no-default-features -- --headless --resolution 28 \
  --temperature 28 --season-days 90 --population 6000 --seed 42 --days 36000 \
  --report-every 0 --save saves/nutrition-century.json
```

Read `insights.history` in the JSON: subtract cumulative cause counters at the
chosen interval endpoints, rather than comparing lifetime percentages. Fresh
saves are required because v7 adds persistent local depletion.
