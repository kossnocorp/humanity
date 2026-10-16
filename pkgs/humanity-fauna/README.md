# humanity-fauna

Owns terrestrial animal populations and their biology. Depends on spherical grid
geometry, flora food-budget types and serialization, not human state, nutrition,
the world or the renderer.

- `src/species.rs`: 14 inspectable species strategies and trait data.
- `src/ecology.rs`: habitat suitability, diets, food consumption, recruitment,
  mortality, human avoidance, movement, dispersal and consolidation.
- `src/hunting.rs`: density/cover/wariness-dependent encounter, trapping and actual
  animal removal. Capture skill and food preparation remain nutrition concerns.
- `src/lib.rs`: `Herd` (also used for flocks/colonies), persisted `FaunaStats` and
  the RNG interface.
- `humanity-world/src/wildlife.rs`: maps world cells into habitat inputs, copies
  consumed food stocks back and maintains derived small-game biomass.

## Species and differing strategies

| Species | Main niche/diet | Recruitment / natural mortality per year | Human tolerance |
|---|---|---:|---:|
| Reindeer | Cold mixed grazer/browser | .32 / .12 | .25 |
| Bison | Open-country grazer | .26 / .10 | .25 |
| Antelope | Warm open-country grazer | .42 / .16 | .35 |
| Deer | Woodland/edge browser | .42 / .14 | .65 |
| Boar | Adaptable omnivore | .95 / .28 | .85 |
| Ibex | Mountain mixed feeder | .30 / .12 | .30 |
| Hare | Fast-recruiting grazer | 2.00 / .85 | .75 |
| Rabbit | Colonial, concealed grazer | 3.00 / 1.15 | .90 |
| Grouse | Woodland seeds/vegetation | 1.80 / .80 | .65 |
| Wild goose | Wetland-associated grazer | .80 / .30 | .45 |
| Beaver | Riparian woody browser | .50 / .18 | .65 |
| Wild horse | Open-country grazer | .25 / .10 | .20 |
| Elk | Large forest-edge browser | .28 / .12 | .40 |
| Mammoth | Slow-recruiting megaherbivore | .075 / .045 | .10 |

These are **prototype population-level assumptions**, not fitted paleobiological
rates or literal litter sizes. Recruitment already incorporates nonbreeding age/sex
classes. There are no individual animal records or explicit predator populations;
natural mortality includes unmodelled predation/disease. Names denote functional
animal types on procedural planets, not a reconstruction of Earth's biogeography.

## Resource and population accounting

`humanity-flora` supplies one sustainable production budget per cell, shared by all
species. Carrying capacity accounts for competing dietary demand and standing
reserves, with no positive empty-pasture floor. Incoming dispersers must fit within
the projected food budget. Species-density limits remain additional upper bounds.

Grazers use low vegetation, browsers woody browse, omnivores fruit/seeds, with
costly substitutions when preferred resources run short. Wildlife shares fruit
with humans. Capacity varies with habitat, climate, area and available food; births
decrease as local same-species density approaches capacity. Recruitment follows
local seasons and personal-population condition; thermal suitability is smooth.

Existing populations can send animals into accessible, suitable neighboring cells.
That transfer creates a daughter **group**, not new animals. Same-species groups
in one cell merge conservatively. Group IDs are monotonic and never reused.
Geese can make short flights across neighboring water cells; other modeled land
dispersal follows land connectivity. Refuge populations can replenish hunted areas.

For every species, saves validate:

`living = initial + births - natural_deaths - harvested`

Small-game food is derived from the live hare/rabbit/grouse/goose/beaver populations.
It does not regenerate independently. Trapping and hunting both subtract actual
animals, including the last animal when an encounter succeeds. Empty populations
cannot reproduce; no continuing generation routine or minimum-population floor
restores an extinct species. A lone remaining animal has no reproductive partner.

## Response to humans

Species differ in spatial avoidance and concealment. Disturbance and actual hunting
raise persisted wariness, which relaxes with time away from people. Dense cover,
rarity, distance and wariness lower encounters. This approximates avoidance and
reduced daytime detectability, **not** an explicit hourly nocturnal activity model.
Human presence alone is not an automatic death event. Tolerant, fast-recruiting
species can persist near people while slow breeders remain vulnerable.

Hunters choose expected obtainable food rather than the first nearby group. Effort,
handling time, existing stores and actual abundance constrain kills. Hunting need
not eliminate every local animal merely because a population occupies a large cell.

`--inspect` exposes `wildlife`, `wildlife_diagnostics`, dispersals and mergers.
Diagnostics distinguish species absent at generation from species subsequently
extinct and report per-species harvest and wariness. Cell field notes show browse,
local species, condition and wariness. Rates, habitat restrictions and response
coefficients are inspectable in this crate; see `docs/research/fauna.md` for evidence
and evaluation boundaries. Fresh v11 saves are required. The biome extension adds
camels (arid browsers), tapirs (tropical-forest browsers) and water buffalo (wetland
grazers), for 17 species. All use the same competition and conservation accounting.
