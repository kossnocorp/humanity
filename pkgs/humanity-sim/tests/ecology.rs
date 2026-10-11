use humanity_core::*;
use humanity_sim::Simulation;

fn config(seed: u64) -> PlanetConfig {
    PlanetConfig {
        width: 64,
        height: 32,
        cell_km: 20.,
        temperature: 28.,
        season_days: 30,
        axial_tilt: 23.4,
        ocean_fraction: 0.6,
        rainfall: 1.,
        population: 6000,
        seed,
    }
}

#[test]
fn climates_and_dense_populations_keep_valid_stocks_and_accounting() {
    for (seed, temperature, rain, population) in [
        (1, 8., 0.2, 1000),
        (9, 42., 0.5, 100_000),
        (24, 28., 3., 6000),
    ] {
        let mut c = config(seed);
        c.temperature = temperature;
        c.rainfall = rain;
        c.population = population;
        let mut w = humanity_world::generate(c).unwrap();
        Simulation::default().advance(&mut w, 240);
        assert_eq!(
            w.population(),
            u64::from(population) + w.totals.births - w.totals.deaths
        );
        for c in &w.cells {
            for stock in [
                c.resources.plants,
                c.resources.fruit,
                c.resources.game,
                c.resources.grazing,
            ] {
                assert!(stock.is_finite() && stock >= 0.);
            }
        }
        for t in &w.tribes {
            let s = &t.specialists;
            assert_eq!(
                s.hunters + s.gatherers + s.healers + s.guides,
                t.people.adults()
            );
            assert!(!w.cells[t.cell as usize].biome.water());
        }
        for h in &w.herds {
            assert!(!w.cells[h.cell as usize].biome.water());
        }
    }
}

#[test]
fn wildlife_moves_and_humans_consume_real_cell_stocks() {
    let mut w = humanity_world::generate(config(42)).unwrap();
    let original_herds = w.herds.clone();
    let mut expected = w.clone();
    expected.day += 1;
    humanity_world::update_environment(&mut expected, false);
    let available: f64 = expected
        .cells
        .iter()
        .map(|c| f64::from(c.resources.edible()))
        .sum();
    let mut sim = Simulation::default();
    sim.step(&mut w);
    let remaining: f64 = w
        .cells
        .iter()
        .map(|c| f64::from(c.resources.edible()))
        .sum();
    assert!(w.totals.gathered > 0.);
    assert!((available - remaining - w.totals.gathered).abs() < 2.);
    sim.advance(&mut w, 119);
    let moved = w
        .herds
        .iter()
        .filter(|h| {
            original_herds
                .iter()
                .any(|o| o.id == h.id && o.cell != h.cell)
        })
        .count();
    assert!(moved > w.herds.len() / 3);
    assert!(w.totals.animals_hunted > 0);
    assert!(w.totals.conflicts > 0 && w.totals.exchanges > 0);
}
