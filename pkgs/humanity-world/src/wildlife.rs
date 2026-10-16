//! World/fauna adapter: ecology itself lives in humanity-fauna.
use humanity_core::*;
use humanity_fauna::{Habitat, ecology::Environment};

pub fn habitats(w: &World) -> Vec<Habitat> {
    let mut people = vec![0_f32; w.cells.len()];
    for t in &w.tribes {
        people[t.cell as usize] += t.headcount() as f32;
        for cell in w.grid.neighbors(t.cell) {
            people[cell as usize] += t.headcount() as f32 * 0.15;
        }
    }
    w.cells
        .iter()
        .enumerate()
        .map(|(i, c)| {
            let area = (w.config.cell_km / 20.).powi(2) * w.grid.area_factor(i as u32);
            Habitat {
                water: c.biome.water(),
                forest: c.biome.woodland(),
                arid: matches!(c.biome, Biome::Desert | Biome::Scrubland | Biome::Steppe),
                tropical_forest: matches!(c.biome, Biome::Rainforest | Biome::Swamp),
                wetland: c.biome.wetland()
                    || c.river > 0.
                    || w.grid
                        .neighbors(i as u32)
                        .any(|id| w.cells[id as usize].biome.water()),
                elevation: c.elevation,
                temperature: c.weather.temperature,
                annual_temperature: w.config.temperature
                    - 52. * w.grid.cooling(i as u32)
                    - c.elevation.max(0) as f32 * 0.0065,
                snow: c.weather.snow,
                area,
                human_pressure: people[i] / (20. * area),
                breeding_season: w.local_season(i as u32) < 2,
                grazing: c.resources.grazing,
                browse: c.resources.browse,
                fruit: c.resources.fruit,
                forage_supply: c.resources.vegetation.wildlife_supply,
            }
        })
        .collect()
}
pub fn initialize(w: &mut World) {
    w.herds = humanity_fauna::seed(
        &habitats(w),
        &mut w.next_herd_id,
        &mut w.rng,
        &mut w.fauna_stats,
    );
    refresh_small_game(w);
}
pub fn advance(w: &mut World) {
    let mut habitats = habitats(w);
    humanity_fauna::advance(
        &mut w.herds,
        Environment {
            grid: &w.grid,
            habitats: &mut habitats,
            radius: w.config.radius_km(),
            cell_km: w.config.cell_km,
            year_days: w.config.year_days(),
            day: w.day,
        },
        &mut w.next_herd_id,
        &mut w.rng,
        &mut w.fauna_stats,
    );
    for (c, habitat) in w.cells.iter_mut().zip(habitats) {
        c.resources.grazing = habitat.grazing;
        c.resources.browse = habitat.browse;
        c.resources.fruit = habitat.fruit;
    }
    refresh_small_game(w);
}
pub fn refresh_small_game(w: &mut World) {
    for c in &mut w.cells {
        c.resources.game = 0.;
    }
    for h in &w.herds {
        if h.species.small() {
            w.cells[h.cell as usize].resources.game += h.animals as f32 * h.species.meat();
        }
    }
}
