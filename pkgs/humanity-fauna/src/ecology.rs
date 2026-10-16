use crate::*;
use humanity_flora::{FLORA, Forage};
use humanity_grid::Grid;

/// Daily environmental/resource inputs. The caller copies consumed stocks back to the world.
#[derive(Clone, Debug)]
pub struct Habitat {
    /// Water cells are not terrestrial population locations.
    pub water: bool,
    /// Woodland habitat available for browsers and concealment.
    pub forest: bool,
    /// River, lake or coastal access for wetland specialists.
    pub wetland: bool,
    pub arid: bool,
    pub tropical_forest: bool,
    /// Elevation in meters.
    pub elevation: i32,
    /// Current temperature, Celsius.
    pub temperature: f32,
    /// Annual mean temperature for initial range and carrying suitability.
    pub annual_temperature: f32,
    /// Snow cover, 0..1.
    pub snow: f32,
    /// Cell area in standard 20 km cell units.
    pub area: f32,
    /// Local/nearby human presence, in groups of twenty people per standard cell.
    pub human_pressure: f32,
    /// Local spring/summer versus autumn/winter reproductive phase.
    pub breeding_season: bool,
    /// Consumable low vegetation, in animal-food units.
    pub grazing: f32,
    /// Consumable woody vegetation/leaves, in animal-food units.
    pub browse: f32,
    /// Consumable fruit/seeds shared with human foragers.
    pub fruit: f32,
    /// Shared sustainable daily production from flora, not a separate budget per species.
    pub forage_supply: Forage,
}
impl Habitat {
    pub fn cover(&self) -> f32 {
        ((if self.forest { 0.75 } else { 0.15 }) + (self.elevation.max(0) as f32 / 5000.).min(0.2))
            .min(0.95)
    }
    pub fn suitability(&self, species: Species, temperature: f32) -> f32 {
        if self.water {
            return 0.;
        }
        let t = species.traits();
        let thermal = 1. / (1. + ((temperature - t.temperature) / t.tolerance).powi(4));
        let niche = match species {
            Species::Camel => {
                if self.arid {
                    1.
                } else {
                    0.02
                }
            }
            Species::Tapir => {
                if self.tropical_forest {
                    1.
                } else {
                    0.02
                }
            }
            Species::WaterBuffalo => {
                if self.wetland {
                    1.
                } else {
                    0.02
                }
            }
            Species::Beaver => {
                if self.wetland && self.forest {
                    1.
                } else {
                    0.02
                }
            }
            Species::WildGoose => {
                if self.wetland {
                    1.
                } else {
                    0.25
                }
            }
            Species::Ibex => {
                if self.elevation > 800 {
                    1.
                } else {
                    0.25
                }
            }
            Species::Deer | Species::Elk | Species::Grouse => {
                if self.forest {
                    1.
                } else {
                    0.5
                }
            }
            Species::Boar => {
                if self.forest {
                    1.
                } else {
                    0.7
                }
            }
            Species::Bison | Species::Antelope | Species::WildHorse | Species::Mammoth
                if self.forest =>
            {
                0.3
            }
            _ => 1.,
        };
        thermal * niche
    }
    pub fn need(&self, species: Species, animals: u32) -> f32 {
        let t = species.traits();
        animals as f32
            * t.food_need
            * (1. + (t.temperature - self.temperature - t.tolerance).max(0.) * 0.01)
    }
    pub fn demand(&self, species: Species, animals: u32) -> Forage {
        species
            .traits()
            .diet
            .weights()
            .scaled(self.need(species, animals))
    }
    /// Capacity allocated to this species from a single, shared local food budget.
    /// Both production and standing reserves constrain it; no positive empty-cell floor.
    pub fn capacity(&self, species: Species, total_demand: Forage, population: u32) -> f32 {
        let t = species.traits();
        let mut support = 0.;
        for (((weight, demand), production), stock) in t
            .diet
            .weights()
            .values()
            .into_iter()
            .zip(total_demand.values())
            .zip(self.forage_supply.values())
            .zip([self.grazing, self.browse, self.fruit])
        {
            if weight > 0. && demand > 0. {
                let efficiency = if weight >= 0.5 { 1. } else { 0.5 };
                support +=
                    weight * production.min(stock / FLORA.reserve_days) * efficiency / demand;
            }
        }
        (population as f32 * support)
            .min(t.density * self.area * self.suitability(species, self.annual_temperature))
    }
    fn accepts(&self, species: Species, demand: Forage, resident: u32, incoming: u32) -> bool {
        let population = resident + incoming;
        let capacity = self.capacity(species, demand + self.demand(species, incoming), population);
        population as f32 <= capacity * 0.65
    }
    fn feed(&mut self, diet: Diet, need: f32) -> f32 {
        let weights = diet.weights().values();
        let mut eaten = 0.;
        for (stock, weight) in [&mut self.grazing, &mut self.browse, &mut self.fruit]
            .into_iter()
            .zip(weights)
        {
            let take = stock.min(need * weight);
            *stock -= take;
            eaten += take;
        }
        // Substitution is possible, but nonpreferred resources require more biomass.
        for (stock, weight) in [&mut self.grazing, &mut self.browse, &mut self.fruit]
            .into_iter()
            .zip(weights)
        {
            if weight == 0. {
                continue;
            }
            let efficiency = if weight >= 0.5 { 1. } else { 0.5 };
            let take = stock.min((need - eaten).max(0.) / efficiency);
            *stock -= take;
            eaten += take * efficiency;
        }
        eaten
    }
}

/// Initial range seeding only; ongoing population recovery must come from reproduction/dispersal.
pub fn seed(
    habitats: &[Habitat],
    next_id: &mut u64,
    rng: &mut impl FaunaRng,
    stats: &mut FaunaStats,
) -> Vec<Herd> {
    let mut groups = Vec::new();
    for (cell, habitat) in habitats.iter().enumerate() {
        let mut demand = Forage::default();
        for species in Species::ALL {
            let quality = habitat.suitability(species, habitat.annual_temperature);
            if quality < 0.25 {
                continue;
            }
            let chance = (if species.small() { 0.14 } else { 0.045 }) * quality * habitat.area;
            if rng.unit() >= chance {
                continue;
            }
            let t = species.traits();
            let desired = (t.group_size as f32 * habitat.area * (0.6 + rng.unit() * 0.8)) as u32;
            let (mut low, mut high) = (0, desired);
            while low < high {
                let mid = low + (high - low).div_ceil(2);
                if habitat.accepts(species, demand, 0, mid) {
                    low = mid;
                } else {
                    high = mid - 1;
                }
            }
            let count = low;
            if count < 2 {
                continue;
            }
            groups.push(Herd {
                id: *next_id as u32,
                cell: cell as u32,
                species,
                animals: count,
                condition: 0.9,
                travel: 0.,
                wariness: 0.,
            });
            *next_id += 1;
            stats.initial[species.index()] += u64::from(count);
            demand = demand + habitat.demand(species, count);
        }
    }
    groups
}

pub struct Environment<'a> {
    /// Spherical topology; no cylindrical seams or unrestricted teleportation.
    pub grid: &'a Grid,
    /// Mutable daily habitat stocks.
    pub habitats: &'a mut [Habitat],
    /// Physical planetary radius in kilometers.
    pub radius: f32,
    /// Mean cell spacing in kilometers.
    pub cell_km: f32,
    /// Simulated days per year.
    pub year_days: u32,
    /// Current simulation day.
    pub day: u64,
}

/// Resource-limited recruitment, avoidance, migration and conservative daughter dispersal.
pub fn advance(
    groups: &mut Vec<Herd>,
    environment: Environment<'_>,
    next_id: &mut u64,
    rng: &mut impl FaunaRng,
    stats: &mut FaunaStats,
) {
    let Environment {
        grid,
        habitats,
        radius,
        cell_km,
        year_days,
        day,
    } = environment;
    let year = year_days as f32;
    let mut occupancy = vec![[0_u32; SPECIES_COUNT]; habitats.len()];
    let mut demand = vec![Forage::default(); habitats.len()];
    for h in groups.iter() {
        occupancy[h.cell as usize][h.species.index()] += h.animals;
        demand[h.cell as usize] =
            demand[h.cell as usize] + habitats[h.cell as usize].demand(h.species, h.animals);
    }
    let mut daughters = Vec::new();
    for h in groups.iter_mut() {
        if h.animals == 0 {
            continue;
        }
        let t = h.species.traits();
        let species = h.species.index();
        let here = h.cell;
        let pressure = habitats[here as usize].human_pressure;
        let alarm = pressure / (1. + pressure) * (1. - t.human_tolerance * 0.5);
        let memory = (-1. / (year * 0.2)).exp();
        h.wariness = (h.wariness * memory + alarm * (1. - memory)).clamp(0., 1.);
        h.travel =
            (h.travel + t.movement * (1. - habitats[here as usize].snow * 0.5)).min(cell_km * 2.5);
        let score = |id: u32| {
            let c = &habitats[id as usize];
            if c.water {
                return f32::NEG_INFINITY;
            }
            let others = occupancy[id as usize][species].saturating_sub(if id == here {
                h.animals
            } else {
                0
            });
            let projected = if id == here {
                demand[id as usize]
            } else {
                demand[id as usize] + c.demand(h.species, h.animals)
            };
            let food = c.grazing + c.browse;
            c.suitability(h.species, c.temperature) * 12.
                + (c.capacity(h.species, projected, others + h.animals)
                    / (others + h.animals).max(1) as f32)
                    .min(2.)
                    * 8.
                + (food / (c.need(h.species, h.animals) * 14.).max(1.)).min(1.) * 4.
                - c.human_pressure * (1. - t.human_tolerance) * 5.
                - c.snow
                    * if h.species == Species::Reindeer {
                        1.
                    } else {
                        3.
                    }
        };
        let mut candidates: Vec<_> = grid.neighbors(here).collect();
        if h.species == Species::WildGoose {
            for water in grid
                .neighbors(here)
                .filter(|&id| habitats[id as usize].water)
            {
                candidates.extend(
                    grid.neighbors(water)
                        .filter(|&id| id != here && !habitats[id as usize].water),
                );
            }
            candidates.sort_unstable();
            candidates.dedup();
        }
        let mut best = here;
        let mut best_score = score(here) + 1.5;
        for &next in &candidates {
            let distance = grid.angular_distance(here, next) * radius;
            if distance > h.travel {
                continue;
            }
            let value = score(next) + rng.unit();
            if value > best_score {
                best = next;
                best_score = value;
            }
        }
        if best != here {
            demand[here as usize] =
                demand[here as usize] - habitats[here as usize].demand(h.species, h.animals);
            demand[best as usize] =
                demand[best as usize] + habitats[best as usize].demand(h.species, h.animals);
            occupancy[here as usize][species] -= h.animals;
            occupancy[best as usize][species] += h.animals;
            h.travel -= grid.angular_distance(here, best) * radius;
            h.cell = best;
        }
        let c = &mut habitats[h.cell as usize];
        let need = c.need(h.species, h.animals);
        let eaten = c.feed(t.diet, need);
        // Larger herbivores buffer seasonal deficits with reserves; small fauna
        // respond faster. No universal two-week starvation spiral for all species.
        let buffer = if h.species.small() { 20. } else { 80. };
        h.condition = (h.condition + (eaten / need.max(0.001) - 0.8) / buffer).clamp(0., 1.);
        let capacity = c.capacity(
            h.species,
            demand[h.cell as usize],
            occupancy[h.cell as usize][species],
        );
        let density = if capacity > 0. {
            occupancy[h.cell as usize][species] as f32 / capacity
        } else {
            1.
        };
        // Recruitment rates already fold in age/sex structure. Applying another
        // fractional breeder penalty to pairs made slow-breeding daughter groups
        // structurally nonviable even in rich, unhunted refuges.
        let mating = f32::from(h.animals >= 2);
        let season = 1.
            + if c.breeding_season {
                t.seasonality
            } else {
                -t.seasonality
            };
        let thermal = 0.25 + 0.75 * c.suitability(h.species, c.temperature);
        let births = rng.expected(
            h.animals as f32 * t.recruitment / year
                * h.condition.powi(2)
                * (1. - density).max(0.)
                * mating
                * season
                * thermal,
        );
        let deaths = rng
            .expected(h.animals as f32 * (t.mortality + (1. - h.condition).powi(3) * 3.) / year)
            .min(h.animals);
        stats.births[species] += u64::from(births);
        stats.natural_deaths[species] += u64::from(deaths);
        demand[h.cell as usize] = demand[h.cell as usize] - c.demand(h.species, h.animals);
        h.animals = h.animals + births - deaths;
        demand[h.cell as usize] = demand[h.cell as usize] + c.demand(h.species, h.animals);
        occupancy[h.cell as usize][species] = occupancy[h.cell as usize][species] + births - deaths;
        // Stagger weekly dispersal decisions; splitting costs real animals and travel capacity.
        if h.animals >= 4
            && (h.animals > t.group_size || density > 0.8)
            && h.condition > 0.65
            && (day + u64::from(h.id)).is_multiple_of(7)
            && *next_id <= u64::from(u32::MAX)
            && rng.unit() < t.dispersal * 7. / year
        {
            let source = h.cell;
            let migrants = (h.animals / 4).max(2).min(h.animals - 1);
            let destination = grid
                .neighbors(source)
                .filter(|&id| {
                    let c = &habitats[id as usize];
                    !c.water
                        && c.suitability(h.species, c.annual_temperature) > 0.2
                        && grid.angular_distance(source, id) * radius <= h.travel
                        && c.accepts(
                            h.species,
                            demand[id as usize],
                            occupancy[id as usize][species],
                            migrants,
                        )
                })
                .max_by(|&a, &b| {
                    let room = |id: u32| {
                        let c = &habitats[id as usize];
                        c.capacity(
                            h.species,
                            demand[id as usize] + c.demand(h.species, migrants),
                            occupancy[id as usize][species] + migrants,
                        ) / (occupancy[id as usize][species] + migrants).max(1) as f32
                            / (1. + habitats[id as usize].human_pressure * (1. - t.human_tolerance))
                    };
                    room(a).total_cmp(&room(b)).then(b.cmp(&a))
                });
            if let Some(destination) = destination {
                h.animals -= migrants;
                demand[source as usize] =
                    demand[source as usize] - habitats[source as usize].demand(h.species, migrants);
                demand[destination as usize] = demand[destination as usize]
                    + habitats[destination as usize].demand(h.species, migrants);
                occupancy[source as usize][species] -= migrants;
                occupancy[destination as usize][species] += migrants;
                daughters.push(Herd {
                    id: *next_id as u32,
                    cell: destination,
                    animals: migrants,
                    travel: 0.,
                    ..h.clone()
                });
                *next_id += 1;
                stats.dispersals += 1;
            }
        }
    }
    groups.extend(daughters);
    groups.retain(|h| h.animals > 0);
    // One local population per species/cell bounds state and restores mating opportunities.
    groups.sort_by_key(|h| (h.cell, h.species, h.id));
    let mut merged: Vec<Herd> = Vec::with_capacity(groups.len());
    for h in groups.drain(..) {
        if let Some(previous) = merged.last_mut()
            && previous.cell == h.cell
            && previous.species == h.species
        {
            let total = previous.animals + h.animals;
            let weight = h.animals as f32 / total as f32;
            previous.condition += (h.condition - previous.condition) * weight;
            previous.wariness += (h.wariness - previous.wariness) * weight;
            previous.travel = previous.travel.min(h.travel);
            previous.animals = total;
            stats.mergers += 1;
        } else {
            merged.push(h);
        }
    }
    *groups = merged;
}
