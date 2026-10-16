use humanity_core::{Biome, PlanetConfig};
use humanity_world::{classify_biome, generate};

#[test]
fn climate_and_drainage_select_distinct_communities() {
    for (t, height, moisture, river, expected) in [
        (28., 200, 0.1, 0., Biome::Desert),
        (28., 200, 0.9, 0., Biome::Rainforest),
        (28., 100, 0.9, 30., Biome::Swamp),
        (12., 100, 0.7, 30., Biome::Marsh),
        (12., 900, 0.7, 30., Biome::Forest),
        (10., 200, 0.28, 0., Biome::Steppe),
        (25., 200, 0.28, 0., Biome::Scrubland),
        (-5., 100, 0.9, 30., Biome::Tundra),
    ] {
        assert_eq!(classify_biome(t, height, moisture, river), expected);
    }
}

#[test]
fn generated_world_contains_dry_tropical_and_wetland_habitats() {
    let world = generate(PlanetConfig {
        resolution: 28,
        temperature: 28.,
        cell_km: 20.,
        axial_tilt: 23.5,
        ocean_fraction: 0.65,
        rainfall: 1.,
        population: 600,
        seed: 42,
        season_days: 90,
    })
    .unwrap();
    for biome in [
        Biome::Desert,
        Biome::Rainforest,
        Biome::Swamp,
        Biome::Marsh,
        Biome::Steppe,
        Biome::Scrubland,
    ] {
        assert!(
            world.cells.iter().any(|c| c.biome == biome),
            "missing {biome:?}"
        );
    }
    for c in world.cells.iter().filter(|c| c.biome.wetland()) {
        assert!(!c.biome.water());
        assert!(c.resources.aquatic.freshwater_fish > 0.);
        assert!(c.biome.travel_cost() > Biome::Grassland.travel_cost());
    }
}
