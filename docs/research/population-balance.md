# Population replacement, resilience and extinction

Research-only audit, 2026-10-14. **No simulation code or parameter changes.**
Goal: science-informed populations that can grow/recover under viable conditions,
fluctuate with ecological limits, and sometimes go extinct—not guaranteed survival
and not consistent extinction across ordinary worlds. Indefinite exponential
growth on a finite planet is not the goal.

## What “background mortality” currently means

`humanity-sim/src/lib.rs::demography` applies:

- Births: `adult_females × 0.16 × health² / year_days` each day.
- General deaths: `population × [0.018 / year_days + 0.025 × (1-health)³]`.
- Deaths from that combined roll are attributed proportionally to **background**
  and **nutrition/poor health** using a separate instrumentation RNG.
- Elders subsequently face an additional `0.08 / year_days` mortality risk.
- Conflict casualties are resolved separately in encounters.

Thus background is **an age-independent 1.8% annual hazard**, including at perfect
health. It is not a pathogen, infection, accident, or medical diagnosis. It is a
placeholder residual hazard for otherwise unmodelled mortality. At full health,
elders face approximately 9.8% annual combined background/elder hazard (not exactly
9.8% annual death probability). Infectious disease is not explicitly simulated.
“Nutrition / poor health” is also broader than starvation: injuries/conflicts and
water travel can reduce the same health scalar.

At 360 days/year, holding health constant at .9 adds an annualized poor-health
hazard of .009; at .8 it adds .072, while fertility falls to 81% or 64% of maximum.
Actual health changes daily, so these are illustrative hazard calculations, not
life-table estimates. This coupling suppresses births as well as increasing deaths.
The poor-health coefficient is per day, while background and fertility are per
year; calendar-length comparisons therefore require special care.

## Unchanged-run findings

Continued existing v7 worlds: resolution 28, 20 km cells, temperature 28, 90-day
seasons, initial population 6,000. No parameter sweeps or edited-world experiments.
Rates below use summed daily population exposure, not endpoint population.

| Seed | Interval (years) | Births / 1,000 person-years | Deaths / 1,000 person-years | Population at end |
|---|---|---:|---:|---:|
| 42 | 100–200 | 37.66 | 53.05 | 225 |
| 42 | 200–300 | 34.00 | 56.79 | 50 |
| 7 | 200–300 | 34.28 | 48.35 | 45 |
| 123 | 200–300 | 40.92 | 51.48 | 64 |

Seed 42's last living sample was day 145,081, about year 403; it was extinct by
year 500. Its years 100–200 death rates per 1,000 person-years were 21.41
nutrition/poor-health, 18.71 background, 9.59 elder, and 3.35 conflict. Years
200–300: 25.54, 20.32, 10.55, and .39, respectively.

This is a **persistent replacement deficit**, not merely an unlucky terminal
event or ongoing high conflict. Three seeds support the user's observation;
this small sample does not establish the extinction probability across all worlds.

An ecological warning: seed 42 had only eight herds at year 30, three at year 100,
and two from year 200 onward. Those last herds still grew (998 animals at year
200, 1,356 at year 500), so total prey numbers and spatial access to prey are
different issues. Herd entities migrate and reproduce, but do not bud new herds
to recolonize a large world. This is a candidate contributor, not yet isolated by
a controlled experiment. At year 30, 24 of 86 human bands had fewer than 15 people;
one had no working-age adults. Small-band demographic failure has no band-fusion
or household-recruitment recovery mechanism.

## Why the age model matters even without changing coefficients

The labels “under 15”, “15–49”, and “50+” are misleadingly precise. Actual cohort
transitions remove a random fraction `1/15` of children and `1/35` of adults each
year. These are memoryless residence times, not chronological birthdays. A new
child can enter adulthood immediately; a newly adult person can immediately become
an elder, and others can remain in either stage for many decades.

A continuous-time approximation makes the sensitivity visible. For constant
health 1, age-independent general mortality `mu`, maturation `g=1/15`, adult exit
`a=1/35`, and daughter production `b=.16×.49`, the current stage model has:

`R0 = [g / (g + mu)] × [b / (a + mu)]`

This is expected daughters per newborn female; it excludes fluctuating health,
food feedback, sex-ratio drift and other spatial/stochastic effects. It is not a
fitted estimate for the complete simulation.

| General annual mortality hazard | Current stage-model R0 | Fixed ages 15–50 R0, same fertility/hazard |
|---:|---:|---:|
| .018 | 1.326 | 1.554 |
| .025 | 1.064 | 1.257 |
| .030 | .923 | 1.083 |
| .035 | .809 | .936 |

The fixed-age comparison is `b × exp(-15×mu) × (1-exp(-35×mu))/mu`.
It is an analytical counterfactual, **not an implemented change or promise of
growth**. It demonstrates that age bookkeeping alone can change the sign of
replacement at the same birth/death coefficients. Simply raising births misses
this structural issue. The nominal `.16×35 = 5.6` births over a full reproductive
span is not intrinsically implausible; realized reproduction depends on survival,
health and time actually spent fertile. Do not equate `.16` averaged over all adult
women with a measured interval between consecutive births among parous women.

## Research bearing on balance

- **Gurven & Kaplan (2007)**, [Longevity Among Hunter-Gatherers](https://gurven.anth.ucsb.edu/sites/secure.lsit.ucsb.edu.anth.d7_gurven/files/sitefiles/papers/GurvenKaplan2007pdr.pdf).
  Contemporary forager evidence shows strongly age-dependent mortality, not a
  flat risk: average survival to 15 about 57%, and about 64% of those survivors
  reach 45 in the traditional hunter-gatherer comparison. Their pooled cause
  percentages are not Paleolithic nutrition-death targets; see `deaths.md`.
- **Gurven & Davison (2019)**, [Periodic catastrophes over human evolutionary
  history are necessary to explain the forager population paradox](https://doi.org/10.1073/pnas.1902406116).
  Ten human subsistence populations and five chimpanzee groups. Contemporary
  human populations often have positive growth potential despite substantial
  mortality; long-run stationarity may require combinations of different vital
  rates and occasional crashes. Human mean TFR in the comparison is about six;
  estimated interbirth interval about 3.7 years. This supports recovery between
  setbacks and occasional extinction, not permanent universal decline. It does
  not license arbitrary catastrophic deaths as a balancing device.
- **Kaplan et al. (2000)**, [A Theory of Human Life History Evolution](http://www.unm.edu/~hkaplan/KaplanHillLancasterHurtado_2000_LHEvolution.pdf),
  and [An Evolutionary and Ecological Analysis of Human Fertility, Mating Patterns,
  and Parental Investment](https://ncbi.nlm.nih.gov/books/NBK97292/).
  Human dependency, adult production surpluses and intergenerational transfers
  co-evolve. Young-adult mortality is relatively low; productive lifespans matter
  both for reproduction and provisioning dependants. Age-specific food production
  and consumption should be represented together.
- **Hill & Hurtado (2009)**, [Cooperative breeding in South American
  hunter-gatherers](https://doi.org/10.1098/rspb.2009.1061).
  In two studied societies, breeding pairs received help from about 1.3
  non-reproductive adults on average. Extra-pair provisioning buffers dependency
  and temporary deficits. It does not support a universal claim that grandmothers
  alone provide most surplus.
- **Dyble et al. (2016)**, [Networks of Food Sharing Reveal the Functional
  Significance of Multilevel Sociality in Two Hunter-Gatherer Groups](https://doi.org/10.1016/j.cub.2016.05.064).
  Agta and Mbendjele sharing operates within households, clusters of several
  households, and wider camps. Sharing preferentially supports non-producing
  households and buffers variance in returns.
- **Lewis et al. (2014)**, [High mobility explains demand sharing and enforced
  cooperation in egalitarian hunter-gatherers](https://doi.org/10.1038/ncomms6789).
  A model rather than direct prehistoric evidence: mobility combined with sharing
  supports survival under conditions where isolated families fail. Useful for
  hypotheses about flexible camp membership, not a mortality calibration dataset.
- **Berbesque et al. (2014)**, [Hunter-gatherers have less famine than
  agriculturalists](https://doi.org/10.1098/rsbl.2013.0853).
  Cross-cultural comparison controlling for habitat quality challenges treating
  chronic famine as a defining hunter-gatherer condition. It does not imply an
  absence of shortages or provide a universal direct-starvation death percentage.

## Structural changes to investigate, in priority order

1. **Chronological aggregate age cohorts and a joint life table.** Use annual or
   narrow age bins, still aggregate people rather than person entities. Audit
   survival to reproduction, age-specific fertility, remaining reproductive years,
   and daughter replacement. This corrects the memoryless-age artifact before
   selecting different vital-rate schedules. Later use evidence-informed
   age-specific hazards instead of redistributing deaths merely to force growth.
2. **Age-appropriate consumption and contribution.** Currently every newborn,
   child, adult and elder needs the same ration, while only adults 15–49 acquire
   food or materials. Elders teach but do not forage. This can make families
   unviable despite resources. Represent child growth needs, maternal lactation,
   age/health-dependent contributions and care time; do not make babies cheap
   without accounting for maternal costs, or assume all elders are fully productive.
3. **Need-based sharing and flexible camp membership.** The existing shared band
   food pool already buffers individuals within a band. Inter-band food aid is
   only a side effect of chance friendly encounters, and fission has no reverse
   household joining/fusion process. Conserving transfers, temporary aggregation
   and recruitment can keep a small band with too few workers from entering an
   irreversible health/productivity spiral. They cannot solve synchronized
   region-wide food failure; occasional local extinction remains possible.
4. **Make labor/movement respond to expected net food returns.** Forty percent of
   working adults are assigned hunting regardless of nearby herds; food scores
   still cap regional stock rather than directly predict obtainable calories after
   work and movement costs. Now charging relocation costs without repairing that
   decision model can penalize movements that never should have been chosen.
   Compare staying, local trips and residential relocation using the same work
   budget; allow adaptation toward productive activities. Retain depletion and
   travel costs rather than restore free harvesting.
5. **Audit regional wildlife recovery and spatial food access.** Test herd
   budding/recolonization from surviving animals, habitat connectivity and
   seasonally accessible resources. No spontaneous animal creation or guaranteed
   recovery after global extinction. Distinguish a world-wide shortage from people
   being unable to reach renewable food that still exists elsewhere.
6. **Replace the residual mortality bucket with explicit mechanisms, not an
   additional mortality stack.** Infection/exposure, injury and nutrition could
   interact with susceptibility, care and fertility. When explicit disease replaces
   part of background risk, remove the corresponding placeholder contribution.
   Disease should have ecological/age-dependent exposure rather than be added
   solely to recover a desired pie-chart percentage.

These are hypotheses and a proposed implementation order, not measured effects
from parameter experiments. Fixing realistic survival mechanisms may legitimately
reduce direct nutrition deaths. The previous 30–50% nutrition/poor-health share
was never established as a scientifically correct target and should not be locked
in. Keeping it while also adding infectious mortality would likely worsen decline.

## How to judge a future fix

Measure life tables, age-specific fertility and realized replacement together
with causes per 1,000 person-years. Report cause shares as secondary diagnostics.
Use matched-seed scenarios across climates/densities over centuries, record
extinction frequency and time, recovery after actual resource shocks, and whether
low-density survivors regain positive growth in viable habitats. Include poor
habitats where extinction is expected; do not require every seed to survive.

Run controlled ablations of the mechanisms above before changing rate constants.
A defensible result is recoverable populations in many suitable worlds, plausible
local failures and occasional global extinction—not a hard-coded population floor,
unlimited growth, or a mandatory minimum percentage of any death cause.
