use humanity_core::*;
use humanity_sim::Simulation;

fn config(seed: u64) -> PlanetConfig {
    PlanetConfig {
        resolution: 14,
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
        assert_eq!(w.insights.deaths.checked_total(), Some(w.totals.deaths));
        assert_eq!(
            w.insights.history.back().unwrap().population,
            w.population()
        );
        for c in &w.cells {
            for stock in [
                c.resources.plants,
                c.resources.fruit,
                c.resources.game,
                c.resources.grazing,
                c.resources.aquatic.freshwater_fish,
                c.resources.aquatic.coastal_fish,
                c.resources.aquatic.shellfish,
                c.resources.wood,
                c.resources.fiber,
            ] {
                assert!(stock.is_finite() && stock >= 0.);
            }
        }
        for t in &w.tribes {
            assert!(
                t.knowledge
                    .practitioners
                    .iter()
                    .flatten()
                    .chain(&t.knowledge.prey)
                    .all(|v| v.is_finite() && (0. ..=1.).contains(v))
            );
            let s = &t.specialists;
            assert_eq!(
                s.hunters + s.gatherers + s.fishers + s.healers + s.guides,
                t.people
                    .members
                    .iter()
                    .filter(|h| h.role != Role::Dependent)
                    .count() as u32
            );
            assert!(!w.cells[t.cell as usize].biome.water());
        }
        for h in &w.herds {
            assert!(!w.cells[h.cell as usize].biome.water());
        }
    }
}

#[test]
fn diagnostic_history_is_bounded_and_all_deaths_are_accounted_for() {
    let mut w = humanity_world::generate(config(42)).unwrap();
    Simulation::default().advance(&mut w, 750);
    assert_eq!(w.insights.history.len(), 751);
    assert_eq!(w.insights.history.front().unwrap().day, 0);
    assert_eq!(w.insights.deaths.checked_total(), Some(w.totals.deaths));
    assert!(w.insights.deaths.nutrition > 0);
    assert!(w.insights.deaths.background > 0);
    assert!(w.insights.deaths.aging > 0);
    assert!(w.insights.deaths.conflict > 0);
    assert!(w.insights.recent_deaths.len() <= DEATH_RECORDS);
    for s in &w.insights.history {
        assert_eq!(
            s.population,
            u64::from(w.config.population) + s.births - s.deaths.checked_total().unwrap()
        );
    }
}

#[test]
fn wildlife_moves_and_humans_consume_real_cell_stocks() {
    let mut w = humanity_world::generate(config(42)).unwrap();
    let original_herds = w.herds.clone();
    let mut expected = w.clone();
    expected.day += 1;
    humanity_world::update_environment(&mut expected, false);
    humanity_world::wildlife::advance(&mut expected);
    let available: f64 = expected
        .cells
        .iter()
        .map(|c| f64::from(c.resources.plants + c.resources.fruit))
        .sum();
    let mut sim = Simulation::default();
    sim.step(&mut w);
    let remaining: f64 = w
        .cells
        .iter()
        .map(|c| f64::from(c.resources.plants + c.resources.fruit))
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
    assert!(moved > 0);
    assert!(w.fauna_stats.dispersals > 0);
    for species in Species::ALL {
        let i = species.index();
        let live: u64 = w
            .herds
            .iter()
            .filter(|h| h.species == species)
            .map(|h| u64::from(h.animals))
            .sum();
        assert_eq!(
            live,
            w.fauna_stats.initial[i] + w.fauna_stats.births[i]
                - w.fauna_stats.natural_deaths[i]
                - w.fauna_stats.harvested[i]
        );
    }
    assert!(w.totals.animals_hunted > 0);
    assert!(w.totals.fished > 0.);
    assert!(w.totals.knowledge_exchanges > 0);
    assert!(w.totals.conflicts > 0 && w.totals.exchanges > 0);
}
