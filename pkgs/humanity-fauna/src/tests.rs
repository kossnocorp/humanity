use super::*;
use ecology::Environment;
use humanity_grid::Grid;
use std::collections::BTreeSet;

struct Rng(u64);
impl FaunaRng for Rng {
    fn unit(&mut self) -> f32 {
        self.0 = self
            .0
            .wrapping_mul(6364136223846793005)
            .wrapping_add(1442695040888963407);
        (self.0 >> 40) as f32 / 16_777_216.
    }
}
fn habitat() -> Habitat {
    Habitat {
        water: false,
        forest: true,
        wetland: true,
        elevation: 200,
        temperature: 18.,
        annual_temperature: 18.,
        snow: 0.,
        area: 1.,
        human_pressure: 0.,
        breeding_season: true,
        grazing: 10000.,
        browse: 15000.,
        fruit: 6000.,
        forage_supply: humanity_flora::Forage::new(100., 100., 50.),
    }
}
fn group(species: Species, animals: u32) -> Herd {
    Herd {
        id: 0,
        cell: 0,
        species,
        animals,
        condition: 1.,
        travel: 50.,
        wariness: 0.,
    }
}
fn step(
    groups: &mut Vec<Herd>,
    cells: &mut [Habitat],
    grid: &Grid,
    day: u64,
    next: &mut u64,
    rng: &mut Rng,
    stats: &mut FaunaStats,
) {
    advance(
        groups,
        Environment {
            grid,
            habitats: cells,
            radius: 50.,
            cell_km: 20.,
            year_days: 360,
            day,
        },
        next,
        rng,
        stats,
    );
}

#[test]
fn daughters_recolonize_conservatively_and_extinction_has_no_respawn() {
    let grid = Grid::new(2).unwrap();
    let mut groups = vec![group(Species::Rabbit, 70)];
    let mut stats = FaunaStats::default();
    stats.initial[Species::Rabbit.index()] = 70;
    let mut next = 1;
    let mut rng = Rng(42);
    for day in 1..=3600 {
        let mut cells = vec![habitat(); grid.len()];
        for c in &mut cells {
            c.forest = false;
            c.breeding_season = day % 360 < 180;
        }
        step(
            &mut groups,
            &mut cells,
            &grid,
            day,
            &mut next,
            &mut rng,
            &mut stats,
        );
    }
    assert!(stats.dispersals > 0 && groups.len() > 1);
    let total: u64 = groups.iter().map(|h| u64::from(h.animals)).sum();
    assert_eq!(
        total,
        70 + stats.births[Species::Rabbit.index()] - stats.natural_deaths[Species::Rabbit.index()]
    );
    assert_eq!(
        groups
            .iter()
            .map(|h| (h.cell, h.species))
            .collect::<BTreeSet<_>>()
            .len(),
        groups.len()
    );
    assert_eq!(
        groups.iter().map(|h| h.id).collect::<BTreeSet<_>>().len(),
        groups.len()
    );
    for h in &mut groups {
        h.harvest(u32::MAX, &mut stats);
    }
    for day in 3601..=3960 {
        let mut cells = vec![habitat(); grid.len()];
        step(
            &mut groups,
            &mut cells,
            &grid,
            day,
            &mut next,
            &mut rng,
            &mut stats,
        );
        assert!(
            groups.is_empty(),
            "food recovery cannot recreate an extinct species"
        );
    }
    assert_eq!(stats.harvested[Species::Rabbit.index()], total);
}

#[test]
fn browsers_use_woodland_food_and_warm_generalists_can_reproduce() {
    let grid = Grid::new(2).unwrap();
    let mut groups = vec![
        group(Species::Deer, 20),
        Herd {
            id: 1,
            ..group(Species::Boar, 20)
        },
    ];
    let mut next = 2;
    let mut rng = Rng(100);
    let mut stats = FaunaStats::default();
    for day in 1..=1800 {
        let mut cells = vec![habitat(); grid.len()];
        for c in &mut cells {
            c.grazing = 0.;
            c.temperature = 35.;
            c.annual_temperature = 35.;
            c.breeding_season = day % 360 < 180;
        }
        step(
            &mut groups,
            &mut cells,
            &grid,
            day,
            &mut next,
            &mut rng,
            &mut stats,
        );
        assert!(
            cells
                .iter()
                .all(|c| c.grazing >= 0. && c.browse >= 0. && c.fruit >= 0.)
        );
    }
    assert!(
        groups
            .iter()
            .filter(|h| h.species == Species::Boar)
            .map(|h| h.animals)
            .sum::<u32>()
            > 20
    );
    assert!(
        groups
            .iter()
            .any(|h| h.species == Species::Deer && h.condition > 0.9)
    );
    assert!(stats.births[Species::Deer.index()] > 0);
}

#[test]
fn rare_animals_and_disturbed_refuges_are_harder_to_hunt_without_immunity() {
    let mut h = group(Species::Deer, 80);
    let naive = hunting::encounter(&h, 1., 0.1, 0.);
    for _ in 0..6 {
        h.disturb();
    }
    assert!(hunting::encounter(&h, 1., 0.1, 0.) < naive * 0.5);
    assert!(hunting::encounter(&h, 1., 0.8, 0.) < hunting::encounter(&h, 1., 0.1, 0.));
    h.animals = 1;
    let rare = hunting::encounter(&h, 1., 0.8, 0.);
    assert!(rare > 0. && rare < naive * 0.1);
    let mut stats = FaunaStats::default();
    assert_eq!(h.harvest(1, &mut stats), 1);
    assert_eq!(h.animals, 0);
}

#[test]
fn isolated_last_megaherbivore_can_die_out_without_hunters() {
    let grid = Grid::new(2).unwrap();
    let mut groups = vec![group(Species::Mammoth, 1)];
    let mut next = 1;
    let mut rng = Rng(31);
    let mut stats = FaunaStats::default();
    for day in 1..=72000 {
        let mut cells = vec![habitat(); grid.len()];
        step(
            &mut groups,
            &mut cells,
            &grid,
            day,
            &mut next,
            &mut rng,
            &mut stats,
        );
        if groups.is_empty() {
            break;
        }
    }
    assert!(groups.is_empty());
    assert_eq!(
        stats.births[Species::Mammoth.index()],
        0,
        "no reproductive partner exists"
    );
    assert_eq!(stats.natural_deaths[Species::Mammoth.index()], 1);
}

#[test]
fn slow_breeders_can_persist_and_spread_in_suitable_unhunted_refuges() {
    let grid = Grid::new(2).unwrap();
    // Several independent refuges, rather than requiring one tiny founder group
    // to escape demographic bad luck under every change in carrying capacity.
    let mut groups: Vec<_> = (0..6)
        .map(|id| Herd {
            id,
            cell: id,
            ..group(Species::Mammoth, 6)
        })
        .collect();
    let mut next = 6;
    let mut rng = Rng(42);
    let mut stats = FaunaStats::default();
    for day in 1..=36000 {
        let mut cells = vec![habitat(); grid.len()];
        for c in &mut cells {
            c.forest = false;
            c.temperature = 2.;
            c.annual_temperature = 2.;
            c.breeding_season = day % 360 < 180;
        }
        step(
            &mut groups,
            &mut cells,
            &grid,
            day,
            &mut next,
            &mut rng,
            &mut stats,
        );
    }
    assert!(groups.iter().map(|h| h.animals).sum::<u32>() > 36);
    assert!(stats.dispersals > 0);
    assert_eq!(stats.harvested[Species::Mammoth.index()], 0);
}

#[test]
fn human_disturbance_can_displace_a_shy_group_into_a_refuge() {
    let grid = Grid::new(2).unwrap();
    let mut cells = vec![habitat(); grid.len()];
    cells[0].human_pressure = 8.;
    let mut groups = vec![group(Species::Deer, 30)];
    step(
        &mut groups,
        &mut cells,
        &grid,
        1,
        &mut 1,
        &mut Rng(42),
        &mut FaunaStats::default(),
    );
    assert_ne!(groups[0].cell, 0);
    assert!(grid.neighbors(0).any(|id| id == groups[0].cell));
}

#[test]
fn species_share_one_production_budget_instead_of_each_getting_the_whole_pasture() {
    use humanity_flora::Forage;
    let mut c = habitat();
    c.forage_supply = Forage::new(10., 5., 2.);
    let total = Species::ALL
        .into_iter()
        .fold(Forage::default(), |d, s| d + c.demand(s, 20));
    let supported_need: f32 = Species::ALL
        .into_iter()
        .map(|s| c.capacity(s, total, 20) * c.need(s, 1))
        .sum();
    assert!(supported_need <= c.forage_supply.total() + 0.0001);
    let alone = c.capacity(Species::Rabbit, c.demand(Species::Rabbit, 20), 20);
    let competing = c.capacity(Species::Rabbit, total, 20);
    assert!(competing < alone);
}

#[test]
fn standing_stocks_without_regrowth_do_not_allow_unlimited_births_or_dispersal() {
    let grid = Grid::new(2).unwrap();
    let mut cells = vec![habitat(); grid.len()];
    for c in &mut cells {
        c.forage_supply = humanity_flora::Forage::default();
    }
    let mut groups = vec![group(Species::Rabbit, 70)];
    let mut next = 1;
    let mut rng = Rng(42);
    let mut stats = FaunaStats::default();
    for day in 1..=90 {
        step(
            &mut groups,
            &mut cells,
            &grid,
            day,
            &mut next,
            &mut rng,
            &mut stats,
        );
    }
    assert_eq!(stats.births.iter().sum::<u64>(), 0);
    assert_eq!(stats.dispersals, 0);
    assert!(
        !groups.is_empty(),
        "carrying-capacity limits must not instantly cull existing animals"
    );
    let seeded = seed(&cells, &mut next, &mut rng, &mut stats);
    assert!(seeded.is_empty());
}

#[test]
fn grazers_cannot_substitute_an_entire_fruit_stock_for_absent_pasture() {
    let grid = Grid::new(2).unwrap();
    let mut cells = vec![habitat(); grid.len()];
    for c in &mut cells {
        c.grazing = 0.;
        c.browse = 0.;
        c.forage_supply = humanity_flora::Forage::new(0., 0., 100.);
    }
    let mut groups = vec![group(Species::Bison, 10)];
    let mut stats = FaunaStats::default();
    step(
        &mut groups,
        &mut cells,
        &grid,
        1,
        &mut 1,
        &mut Rng(42),
        &mut stats,
    );
    assert!(groups[0].condition < 1.);
    assert!(cells.iter().all(|c| c.fruit == 6000.));
    assert_eq!(stats.births.iter().sum::<u64>(), 0);
}
