//! Periodic planetary terrain, priority-flood drainage, climate and standing ecology.
use humanity_core::*;
use noise::{NoiseFn, OpenSimplex};
use std::{cmp::Reverse, collections::BinaryHeap, f64::consts::TAU};

pub fn generate(config: PlanetConfig) -> Result<World, String> {
    config.validate()?;
    let noise = OpenSimplex::new(config.seed as u32 ^ (config.seed >> 32) as u32);
    let mut height = Vec::with_capacity(config.len());
    let mut humidity = Vec::with_capacity(config.len());
    for id in 0..config.len() as u32 {
        let lon = f64::from(id % config.width) / f64::from(config.width) * TAU;
        let lat = f64::from(config.latitude(id));
        let sample = |scale: f64, offset: f64| {
            noise.get([
                lon.cos() * scale + offset,
                lon.sin() * scale,
                lat * scale * 2.,
            ]) as f32
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
    drain(&config, &mut cells);
    for (id, cell) in cells.iter_mut().enumerate() {
        if cell.biome.water() {
            continue;
        }
        let t = annual_temperature(&config, id as u32, cell.elevation);
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
        day: 0,
        cells,
        herds: Vec::new(),
        tribes: Vec::new(),
        events: Default::default(),
        totals: Totals::default(),
    };
    update_environment(&mut world, true);
    populate(&mut world)?;
    Ok(world)
}

/// Every land cell has a downstream parent earlier in flood order: no cycles,
/// even in depressions. Accumulation is a single reverse traversal, not path tracing.
fn drain(config: &PlanetConfig, cells: &mut [Cell]) {
    let mut queue = BinaryHeap::new();
    let mut seen = vec![false; cells.len()];
    let mut order = Vec::with_capacity(cells.len());
    let mut runoff: Vec<f32> = cells.iter().map(|c| c.moisture.max(0.1)).collect();
    for (i, cell) in cells.iter().enumerate() {
        if cell.biome.water() {
            seen[i] = true;
            queue.push(Reverse((0, i as u32)));
        }
    }
    while let Some(Reverse((filled, id))) = queue.pop() {
        order.push(id);
        for n in config.neighbors(id) {
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

fn annual_temperature(config: &PlanetConfig, id: CellId, elevation: i32) -> f32 {
    config.temperature
        - 52. * config.latitude(id).abs().powf(1.35)
        - elevation.max(0) as f32 * 0.0065
}

/// Daily weather varies smoothly across space; opposing hemispheres have opposite seasons.
/// The ecology uses actual snow, water and temperature, rather than the season's name.
pub fn update_environment(world: &mut World, initialize: bool) {
    let c = &world.config;
    let phase = (world.day % u64::from(c.year_days())) as f32 / c.year_days() as f32
        * std::f32::consts::TAU;
    let seasonal = phase.sin() * c.axial_tilt * 0.65;
    let area = (c.cell_km / 20.).powi(2);
    // Cache row/column terms to avoid trigonometry for every cell every day.
    let rows: Vec<_> = (0..c.height)
        .map(|y| {
            let lat = c.latitude(y * c.width);
            (
                c.temperature - 52. * lat.abs().powf(1.35) + seasonal * lat,
                (lat * 13. + world.day as f32 * 0.09).sin(),
            )
        })
        .collect();
    let cols: Vec<_> = (0..c.width)
        .map(|x| {
            (x as f32 / c.width as f32 * std::f32::consts::TAU * 3. + world.day as f32 * 0.13).sin()
        })
        .collect();
    for (id, cell) in world.cells.iter_mut().enumerate() {
        let (base, front) = rows[id / c.width as usize];
        let storm = (front * 0.4 + cols[id % c.width as usize] * 0.6).max(0.);
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
        if cell.biome.water() {
            continue;
        }
        let warmth = ((t + 8.) / 25.).clamp(0.02, 1.) * ((55. - t) / 15.).clamp(0.05, 1.);
        let growing = warmth * (0.25 + w.soil_moisture * 0.75) * (1. - w.snow * 0.9);
        let capacity = cell.biome.fertility() * area;
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
        if !world.rng.chance(cell.biome.fertility() * 0.10) {
            continue;
        }
        let species = if cell.weather.temperature < 7. {
            Species::Reindeer
        } else if cell.weather.temperature > 22. {
            Species::Antelope
        } else {
            Species::Bison
        };
        world.herds.push(Herd {
            id: world.herds.len() as u32,
            cell: id,
            species,
            animals: 40 + world.rng.index(140) as u32,
            condition: 0.85,
            travel: 0.,
        });
    }
    let band = (world.config.population / 20_000).max(60);
    let mut remaining = world.config.population;
    while remaining > 0 {
        let n = remaining.min(band + world.rng.index((band / 2).max(1) as usize) as u32);
        remaining -= n;
        let id = habitable[world.rng.index(habitable.len())];
        let female = [n * 15 / 100, n * 30 / 100, n * 5 / 100];
        let male = [
            n * 15 / 100,
            n * 30 / 100,
            n - female.iter().sum::<u32>() - n * 15 / 100 - n * 30 / 100,
        ];
        let people = Demography { female, male };
        let specialists = Specialists::from_adults(people.adults());
        let x = (id % world.config.width) as f32 / world.config.width as f32 * 12.;
        let y = (id / world.config.width) as f32 / world.config.height as f32 * 6.;
        let mut languages = Vec::new();
        for (dx, wx) in [(0, 1. - x.fract()), (1, x.fract())] {
            for (dy, wy) in [(0, 1. - y.fract()), (1, y.fract())] {
                let share = wx * wy;
                if share > 0. {
                    languages.push(Language {
                        family: (y as u32 + dy) * 12 + (x as u32 + dx) % 12,
                        share,
                    });
                }
            }
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
        });
    }
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;
    fn config() -> PlanetConfig {
        PlanetConfig {
            width: 48,
            height: 32,
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
        let summer = w.cells[8 * 48].weather.temperature;
        let south_winter = w.cells[23 * 48].weather.temperature;
        w.day = 90;
        update_environment(&mut w, true);
        assert!(summer > w.cells[8 * 48].weather.temperature + 10.);
        assert!(south_winter < w.cells[23 * 48].weather.temperature - 10.);
    }
}
