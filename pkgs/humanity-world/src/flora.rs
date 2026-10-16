//! Converts geographic/weather state into flora inputs; all plant growth lives in humanity-flora.
use humanity_core::{Biome, Cell};
use humanity_flora::{Community, Environment};

pub fn advance(cell: &mut Cell, area: f32, year_days: u32, initialize: bool) {
    let community = match cell.biome {
        Biome::Steppe => Community::Steppe,
        Biome::Scrubland => Community::Scrubland,
        Biome::Marsh => Community::Marsh,
        Biome::Swamp => Community::Swamp,
        Biome::Ocean | Biome::Sea | Biome::Lake => Community::Water,
        Biome::Ice => Community::Bare,
        Biome::Grassland => Community::Grassland,
        Biome::Savanna => Community::Savanna,
        Biome::Forest => Community::Forest,
        Biome::Rainforest => Community::Rainforest,
        Biome::Taiga => Community::Taiga,
        Biome::Tundra => Community::Tundra,
        Biome::Desert => Community::Desert,
        Biome::Alpine => Community::Alpine,
    };
    humanity_flora::advance(
        &mut cell.resources.vegetation,
        Environment {
            community,
            fertility: cell.biome.fertility(),
            temperature: cell.weather.temperature,
            moisture: cell.weather.soil_moisture,
            snow: cell.weather.snow,
            area,
            year_days,
        },
        initialize,
    );
}
