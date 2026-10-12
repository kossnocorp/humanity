//! Periodic planetary terrain, priority-flood drainage, climate and standing ecology.
use humanity_core::*;
use humanity_grid::Grid;
use noise::{NoiseFn, OpenSimplex};
use std::{cmp::Reverse, collections::BinaryHeap};

pub fn generate(config: PlanetConfig) -> Result<World, String> {
    config.validate()?;
    let grid = Grid::new(config.resolution)?;
    let noise = OpenSimplex::new(config.seed as u32 ^ (config.seed >> 32) as u32);
    let mut height = Vec::with_capacity(config.len());
    let mut humidity = Vec::with_capacity(config.len());
    for id in 0..config.len() as u32 {
        let p = grid.direction(id).as_dvec3();
        let sample = |scale: f64, offset: f64| {
            noise.get([p.x * scale + offset, p.y * scale, p.z * scale]) as f32
        };
        let h = sample(1.3, 0.) * 0.65
            + sample(3., 0.) * 0.24
            + sample(7., 0.) * 0.08
            + sample(17., 0.) * 0.03;
        height.push(h);
        humidity.push((0.5 + sample(2.5, 30.) * 0.6).clamp(0.05, 1.));
    }
    let mut sorted = height.clone();
    sorted.sort_unstable_by(f32::total_cmp);
    let sea_level = sorted[(sorted.len() as f32 * config.ocean_fraction) as usize];
    let mut cells: Vec<_> = height
        .iter()
        .enumerate()
        .map(|(i, h)| {
            let elevation = ((h - sea_level) * 6500.) as i32;
            let moisture = (humidity[i] * config.rainfall).clamp(0., 1.);
            Cell {
                elevation,
                biome: if elevation <= 0 {
                    Biome::Ocean
                } else {
                    Biome::Grassland
                },
                moisture,
                drainage: None,
                river: 0.,
                weather: Weather {
                    temperature: 0.,
                    precipitation: 0.,
                    snow: 0.,
                    soil_moisture: moisture,
                },
                resources: Resources {
                    plants: 0.,
                    fruit: 0.,
                    game: 0.,
                    grazing: 0.,
                    aquatic: AquaticLife::default(),
                    wood: 0.,
                    fiber: 0.,
                    toolstone: {
                        let p = grid.direction(i as u32).as_dvec3();
                        ((noise.get([p.x * 9. + 7., p.y * 9., p.z * 9.]) as f32 + 0.2) * 1.4)
                            .clamp(0., 1.)
                    },
                },
            }
        })
        .collect();
    // Ocean-connected shallow margins represent regional seas.
    for cell in &mut cells {
        if cell.elevation <= 0 && cell.elevation > -650 {
            cell.biome = Biome::Sea;
        }
    }
    drain(&grid, &mut cells);
    for (id, cell) in cells.iter_mut().enumerate() {
        if cell.biome.water() {
            continue;
        }
        let t = annual_temperature(&config, &grid, id as u32, cell.elevation);
        cell.moisture = (cell.moisture + (cell.river / 100.).min(0.25)).min(1.);
        cell.biome = if t < -12. {
            Biome::Ice
        } else if cell.elevation > 2400 {
            Biome::Alpine
        } else if t < -2. {
            Biome::Tundra
        } else if t < 7. {
            Biome::Taiga
        } else if cell.moisture < 0.22 {
            Biome::Desert
        } else if t > 22. && cell.moisture > 0.67 {
            Biome::Rainforest
        } else if t > 19. && cell.moisture < 0.5 {
            Biome::Savanna
        } else if cell.moisture > 0.56 {
            Biome::Forest
        } else {
            Biome::Grassland
        };
    }
    let mut world = World {
        version: SAVE_VERSION,
        rng: Rng::new(config.seed),
        config,
        grid,
        day: 0,
        cells,
        herds: Vec::new(),
        tribes: Vec::new(),
        next_band_id: 0,
        cultures: Vec::new(),
        sites: Vec::new(),
        next_site_id: 0,
        events: Default::default(),
        totals: Totals::default(),
        insights: PopulationInsights::default(),
    };
    update_environment(&mut world, true);
    populate(&mut world)?;
    initialize_cultures(&mut world, true);
    world.record_population();
    Ok(world)
}

/// Every land cell has a downstream parent earlier in flood order: no cycles,
/// even in depressions. Accumulation is a single reverse traversal, not path tracing.
fn drain(grid: &Grid, cells: &mut [Cell]) {
    let mut queue = BinaryHeap::new();
    let mut seen = vec![false; cells.len()];
    let mut order = Vec::with_capacity(cells.len());
    let mut runoff: Vec<f32> = cells
        .iter()
        .enumerate()
        .map(|(id, c)| c.moisture.max(0.1) * grid.area_factor(id as u32))
        .collect();
    for (i, cell) in cells.iter().enumerate() {
        if cell.biome.water() {
            seen[i] = true;
            queue.push(Reverse((0, i as u32)));
        }
    }
    while let Some(Reverse((filled, id))) = queue.pop() {
        order.push(id);
        for n in grid.neighbors(id) {
            let i = n as usize;
            if seen[i] {
                continue;
            }
            seen[i] = true;
            let cell = &mut cells[i];
            cell.drainage = Some(id);
            if filled - cell.elevation > 90 {
                cell.biome = Biome::Lake;
            }
            queue.push(Reverse((filled.max(cell.elevation), n)));
        }
    }
    for &id in order.iter().rev() {
        if let Some(to) = cells[id as usize].drainage {
            runoff[to as usize] += runoff[id as usize];
        }
    }
    for (cell, flow) in cells.iter_mut().zip(runoff) {
        if cell.drainage.is_some() && flow > 5. {
            cell.river = flow;
        }
    }
}

fn annual_temperature(config: &PlanetConfig, grid: &Grid, id: CellId, elevation: i32) -> f32 {
    config.temperature - 52. * grid.cooling(id) - elevation.max(0) as f32 * 0.0065
}

/// Daily weather varies smoothly across space; opposing hemispheres have opposite seasons.
/// The ecology uses actual snow, water and temperature, rather than the season's name.
pub fn update_environment(world: &mut World, initialize: bool) {
    let c = &world.config;
    let phase = (world.day % u64::from(c.year_days())) as f32 / c.year_days() as f32
        * std::f32::consts::TAU;
    let seasonal = phase.sin() * c.axial_tilt * 0.65;
    let area = (c.cell_km / 20.).powi(2);
    // Smooth 3D pressure fronts cross all face seams and both poles. Only three
    // trigonometric operations per day; static latitude terms live in the grid.
    let day = (world.day % 1_000_000) as f32;
    let (wind_sin, wind_cos) = (day * 0.13).sin_cos();
    let front = (day * 0.09).sin();
    for (id, cell) in world.cells.iter_mut().enumerate() {
        let p = world.grid.direction(id as u32);
        let base = c.temperature - 52. * world.grid.cooling(id as u32)
            + seasonal * world.grid.latitude(id as u32);
        let storm = ((p.x * wind_cos + p.z * wind_sin) * 0.7 + (p.y * front).abs() * 0.3).max(0.);
        let t = base - cell.elevation.max(0) as f32 * 0.0065 + (storm - 0.3) * 5.;
        let precipitation = storm * cell.moisture * 8. * c.rainfall;
        let w = &mut cell.weather;
        w.temperature = t;
        w.precipitation = precipitation;
        let melt = if t > 0. { w.snow.min(t * 0.025) } else { 0. };
        w.snow = (w.snow + if t < 0. { precipitation * 0.025 } else { -melt }).clamp(0., 1.);
        w.soil_moisture = (w.soil_moisture
            + precipitation * 0.007
            + melt * 0.1
            + if cell.river > 0. { 0.007 } else { 0. }
            - (0.003 + t.max(0.) * 0.00025))
            .clamp(0.02, 1.);
        renew_subsistence(cell, area * world.grid.area_factor(id as u32), initialize);
        if cell.biome.water() {
            continue;
        }
        let warmth = ((t + 8.) / 25.).clamp(0.02, 1.) * ((55. - t) / 15.).clamp(0.05, 1.);
        let growing =
            warmth * (0.25 + cell.weather.soil_moisture * 0.75) * (1. - cell.weather.snow * 0.9);
        let capacity = cell.biome.fertility() * area * world.grid.area_factor(id as u32);
        let fruiting = ((t - 8.) / 16.).clamp(0., 1.);
        let targets = [
            capacity * (0.15 + growing) * 10000.,
            capacity * growing * fruiting * 6000.,
            capacity * (0.3 + growing) * 3500.,
            capacity * growing * 14000.,
        ];
        let r = &mut cell.resources;
        for (stock, target, renewal) in [
            (&mut r.plants, targets[0], 0.025),
            (&mut r.fruit, targets[1], 0.045),
            (&mut r.game, targets[2], 0.006),
            (&mut r.grazing, targets[3], 0.035),
        ] {
            if initialize {
                *stock = target;
            } else {
                *stock = (*stock + (target - *stock) * renewal).max(0.);
            }
        }
    }
}

fn populate(world: &mut World) -> Result<(), String> {
    let land: Vec<_> = world
        .cells
        .iter()
        .enumerate()
        .filter(|(_, c)| !c.biome.water())
        .map(|(i, _)| i as u32)
        .collect();
    let habitable: Vec<_> = land
        .iter()
        .copied()
        .filter(|&i| world.cells[i as usize].biome.fertility() >= 0.16)
        .collect();
    if habitable.is_empty() {
        return Err(
            "this climate generated no habitable land; increase temperature or rainfall".into(),
        );
    }
    for &id in &land {
        let cell = &world.cells[id as usize];
        if !world
            .rng
            .chance(cell.biome.fertility() * 0.10 * world.grid.area_factor(id))
        {
            continue;
        }
        let species = if cell.elevation > 1400 {
            Species::Ibex
        } else if matches!(cell.biome, Biome::Forest | Biome::Taiga | Biome::Rainforest) {
            if world.rng.chance(0.55) {
                Species::Deer
            } else {
                Species::Boar
            }
        } else if cell.weather.temperature < 7. {
            Species::Reindeer
        } else if cell.weather.temperature > 22. {
            Species::Antelope
        } else {
            Species::Bison
        };
        let (minimum, spread) = match species {
            Species::Boar => (8, 32),
            Species::Ibex => (12, 45),
            Species::Deer => (15, 85),
            _ => (40, 140),
        };
        world.herds.push(Herd {
            id: world.herds.len() as u32,
            cell: id,
            species,
            animals: minimum + world.rng.index(spread) as u32,
            condition: 0.85,
            travel: 0.,
        });
    }
    let mut remaining = world.config.population;
    while remaining > 0 {
        let id = habitable[world.rng.index(habitable.len())];
        let camp_tradition = initial_camp_tradition(world, id, world.tribes.len() as u32);
        let target = camp_tradition.normal_limit();
        let n = remaining
            .min(
                (target as f32 * SOCIAL.initial_size_min_fraction) as u32
                    + world.rng.index(
                        ((target as f32 * SOCIAL.initial_size_spread_fraction) as usize).max(1),
                    ) as u32,
            )
            .max(1);
        remaining -= n;
        let female = [n * 15 / 100, n * 30 / 100, n * 5 / 100];
        let male = [
            n * 15 / 100,
            n * 30 / 100,
            n - female.iter().sum::<u32>() - n * 15 / 100 - n * 30 / 100,
        ];
        let people = Demography { female, male };
        let specialists = Specialists::from_adults(people.adults());
        // Language affinity follows spherical distance, including across poles.
        let p = world.grid.direction(id);
        let mut languages: Vec<_> = (0..12)
            .map(|family| Language {
                family,
                share: 1. / (0.03 + 1. - p.dot(world.grid.direction(family))).powi(2),
            })
            .collect();
        languages.sort_by(|a, b| b.share.total_cmp(&a.share).then(a.family.cmp(&b.family)));
        languages.truncate(4);
        let total: f32 = languages.iter().map(|l| l.share).sum();
        for l in &mut languages {
            l.share /= total;
        }
        world.tribes.push(Tribe {
            id: world.tribes.len() as u32,
            cell: id,
            people,
            specialists,
            languages,
            food: n as f32 * 8.,
            health: 1.,
            travel: 0.,
            hunting_skill: 0.35 + world.rng.unit() * 0.35,
            knowledge: initial_knowledge(world, id, world.tribes.len() as u32),
            equipment: Equipment::default(),
            fishing_effort: 0.,
            camp_tradition,
            camp_pressure_days: 0,
            parent_band: None,
            culture: 0,
            isolation_days: 0,
            last_cultural_contact: 0,
            contact_culture: None,
            contact_days: 0,
            last_foreign_contact: 0,
            residence_days: 0,
        });
    }
    world.next_band_id = world.tribes.len() as u64;
    Ok(())
}

/// Ecology biases customs but does not determine them. Isolated RNG also makes
/// old-save initialization independent of the gameplay random stream.
pub fn initial_camp_tradition(world: &World, cell: CellId, band: u32) -> CampTradition {
    initial_culture_variant(world, cell, band).rules().camp
}

pub fn initial_culture_variant(world: &World, cell: CellId, band: u32) -> CultureVariant {
    let mut rng = Rng::new(world.config.seed ^ u64::from(band).wrapping_mul(0xd6e8feb86659fd93));
    let waterside = world.cells[cell as usize].river > 0.
        || world
            .grid
            .neighbors(cell)
            .any(|n| world.cells[n as usize].biome.water());
    let weight = |variant: CultureVariant| {
        let rules = variant.rules();
        if waterside {
            rules.waterside_weight
        } else {
            rules.inland_weight
        }
    };
    let total: u32 = CultureVariant::ALL.into_iter().map(weight).sum();
    let mut roll = rng.index(total as usize) as u32;
    for variant in CultureVariant::ALL {
        if roll < weight(variant) {
            return variant;
        }
        roll -= weight(variant);
    }
    CultureVariant::CooperativeForagers
}

/// Initial identities span multiple camps with a shared language and strategy.
/// Upgrades preserve existing knowledge and the gameplay RNG.
pub fn initialize_cultures(world: &mut World, seed_knowledge: bool) {
    let mut groups = std::collections::BTreeMap::new();
    for i in 0..world.tribes.len() {
        let t = &world.tribes[i];
        let variant = initial_culture_variant(world, t.cell, t.id);
        let language = t
            .languages
            .iter()
            .max_by(|a, b| a.share.total_cmp(&b.share))
            .map_or(0, |l| l.family);
        let id = *groups
            .entry((language, variant as usize))
            .or_insert_with(|| {
                let id = world.cultures.len() as u32;
                world.cultures.push(Culture {
                    id,
                    name: format!("Tradition {}", id + 1),
                    variant,
                    parents: Vec::new(),
                    founded: world.day,
                });
                id
            });
        let t = &mut world.tribes[i];
        t.culture = id;
        t.camp_tradition = variant.rules().camp;
        t.last_cultural_contact = world.day;
        if seed_knowledge {
            for (skill, floor) in [
                (Skill::Fishing, variant.rules().fishing_floor),
                (Skill::Preservation, variant.rules().preservation_floor),
            ] {
                if t.knowledge.level(skill) < floor {
                    t.knowledge.seed(skill, floor);
                }
            }
        }
    }
}

/// Also used for v2 upgrades; a separate seed avoids changing gameplay RNG.
pub fn initial_knowledge(world: &World, cell: CellId, band: u32) -> Knowledge {
    let c = &world.cells[cell as usize];
    let water = c.river > 0.
        || world
            .grid
            .neighbors(cell)
            .any(|n| world.cells[n as usize].biome.water());
    let mut rng = Rng::new(world.config.seed ^ u64::from(band).wrapping_mul(0x9e3779b97f4a7c15));
    let mut k = Knowledge::default();
    for skill in [
        Skill::Knapping,
        Skill::Tracking,
        Skill::PlantProcessing,
        Skill::OralTradition,
        Skill::Healing,
    ] {
        k.seed(skill, 0.2 + rng.unit() * 0.5);
    }
    k.seed(Skill::Cordage, 0.15 + rng.unit() * 0.5);
    for skill in [
        Skill::Trapping,
        Skill::DriveHunting,
        Skill::Preservation,
        Skill::Clothing,
    ] {
        if rng.chance(0.65) {
            k.seed(skill, 0.15 + rng.unit() * 0.45);
        }
    }
    if c.weather.temperature < 8. {
        k.seed(Skill::Clothing, 0.65);
    }
    if water {
        k.seed(Skill::Fishing, 0.3 + rng.unit() * 0.4);
        if rng.chance(0.65) {
            k.seed(Skill::Watercraft, 0.3 + rng.unit() * 0.4);
            k.seed(Skill::Navigation, 0.15 + rng.unit() * 0.4);
        }
    }
    for species in Species::ALL {
        k.prey[species.index()] = (0.15 + species.habitat(c) * 0.015).clamp(0.02, 0.5);
    }
    k
}

/// Aquatic biomass and craft plants renew independently of terrestrial food.
pub fn renew_subsistence(cell: &mut Cell, area: f32, initialize: bool) {
    let t = cell.weather.temperature;
    let productivity = ((t + 12.) / 25.).clamp(0.08, 1.) * ((48. - t) / 15.).clamp(0.05, 1.);
    let fresh = if cell.biome == Biome::Lake {
        1.
    } else {
        (cell.river / 30.).min(1.)
    };
    let coast = match cell.biome {
        Biome::Sea => 1.,
        Biome::Ocean => 0.12,
        _ => 0.,
    };
    let forest = if matches!(cell.biome, Biome::Forest | Biome::Taiga | Biome::Rainforest) {
        1.
    } else {
        cell.biome.fertility() * 0.2
    };
    let r = &mut cell.resources;
    for (stock, capacity, rate) in [
        (
            &mut r.aquatic.freshwater_fish,
            fresh * 5000. * productivity,
            0.012,
        ),
        (
            &mut r.aquatic.coastal_fish,
            coast * 7000. * productivity,
            0.009,
        ),
        (
            &mut r.aquatic.shellfish,
            if cell.biome == Biome::Sea {
                2500. * productivity
            } else {
                fresh * 400. * productivity
            },
            0.006,
        ),
        (&mut r.wood, forest * 2500., 0.002),
        (
            &mut r.fiber,
            (cell.biome.fertility() + fresh * 0.5) * 1000.,
            0.02,
        ),
    ] {
        let target = area * capacity;
        *stock = if initialize {
            target
        } else {
            (*stock + (target - *stock) * rate).max(0.)
        };
    }
}

pub fn upgrade_subsistence(world: &mut World) {
    let area = (world.config.cell_km / 20.).powi(2);
    for (id, c) in world.cells.iter_mut().enumerate() {
        renew_subsistence(c, area * world.grid.area_factor(id as u32), true);
        c.resources.toolstone = Rng::new(world.config.seed ^ id as u64).unit();
    }
    for i in 0..world.tribes.len() {
        let t = &world.tribes[i];
        let k = initial_knowledge(world, t.cell, t.id);
        world.tribes[i].knowledge = k;
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    fn config() -> PlanetConfig {
        PlanetConfig {
            resolution: 12,
            cell_km: 20.,
            temperature: 28.,
            season_days: 30,
            axial_tilt: 23.4,
            ocean_fraction: 0.6,
            rainfall: 1.,
            population: 2000,
            seed: 42,
        }
    }
    #[test]
    fn drainage_reaches_water_without_cycles_and_population_is_conserved() {
        let w = generate(config()).unwrap();
        assert_eq!(w.population(), 2000);
        assert!(w.cells.iter().any(|c| c.river > 0.));
        for (i, c) in w.cells.iter().enumerate() {
            if c.biome.water() {
                continue;
            }
            let mut pos = i;
            let mut steps = 0;
            while !w.cells[pos].biome.water() {
                pos = w.cells[pos].drainage.unwrap() as usize;
                steps += 1;
                assert!(steps < w.cells.len());
            }
        }
    }
    #[test]
    fn hemispheres_have_opposite_summers() {
        let mut w = generate(config()).unwrap();
        w.day = 30;
        update_environment(&mut w, true);
        let north = (0..w.cells.len())
            .find(|&i| w.grid.latitude(i as u32) > 0.55)
            .unwrap();
        let south = (0..w.cells.len())
            .find(|&i| w.grid.latitude(i as u32) < -0.55)
            .unwrap();
        let summer = w.cells[north].weather.temperature;
        let south_winter = w.cells[south].weather.temperature;
        w.day = 90;
        update_environment(&mut w, true);
        assert!(summer > w.cells[north].weather.temperature + 10.);
        assert!(south_winter < w.cells[south].weather.temperature - 10.);
    }
}
