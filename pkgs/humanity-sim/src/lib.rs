//! Daily, deterministic aggregate simulation. Sorted sparse spatial indices avoid
//! all-pairs agent scans and per-cell allocations. Scratch is not persistent state.
use humanity_core::*;
mod social;
mod subsistence;

#[derive(Default)]
struct SpatialIndex {
    entries: Vec<(CellId, usize)>,
}
impl SpatialIndex {
    fn rebuild(&mut self, positions: impl Iterator<Item = CellId>) {
        self.entries.clear();
        self.entries
            .extend(positions.enumerate().map(|(i, cell)| (cell, i)));
        self.entries.sort_unstable();
    }
    fn at(&self, cell: CellId) -> &[(CellId, usize)] {
        let start = self.entries.partition_point(|&(c, _)| c < cell);
        let end = self.entries.partition_point(|&(c, _)| c <= cell);
        &self.entries[start..end]
    }
}

#[derive(Default)]
pub struct Simulation {
    herds: SpatialIndex,
    tribes: SpatialIndex,
}
impl Simulation {
    pub fn advance(&mut self, world: &mut World, days: u64) {
        for _ in 0..days {
            self.step(world);
        }
    }

    pub fn step(&mut self, world: &mut World) {
        world.day += 1;
        for t in &mut world.tribes {
            t.refresh(world.day);
        }
        humanity_world::update_environment(world, false);
        humanity_world::wildlife::advance(world);
        self.herds.rebuild(world.herds.iter().map(|h| h.cell));
        self.tribes.rebuild(world.tribes.iter().map(|t| t.cell));
        self.move_tribes(world);
        subsistence::daily(world);
        self.feed_tribes(world);
        self.tribes.rebuild(world.tribes.iter().map(|t| t.cell));
        self.contacts(world);
        demography(world);
        social::daily(world);
        social::fission(world);
        world.herds.retain(|h| h.animals > 0);
        humanity_world::wildlife::refresh_small_game(world);
        world.tribes.retain(|t| t.headcount() > 0);
        world.record_population();
    }

    fn move_tribes(&self, world: &mut World) {
        let mut events = Vec::new();
        let radius = world.config.radius_km();
        let mut cultural_sites = vec![Vec::new(); world.cultures.len()];
        let mut founded_sites = std::collections::BTreeMap::<u32, Vec<_>>::new();
        for site in &world.sites {
            if site.kind == SiteKind::FishingSettlement
                || site.known_seasons & (1 << world.local_season(site.cell)) != 0
            {
                cultural_sites[site.culture as usize].push(site);
                founded_sites
                    .entry(site.founder_band)
                    .or_default()
                    .push(site);
            }
        }
        let populations: Vec<_> = world.tribes.iter().map(Tribe::headcount).collect();
        for index in 0..world.tribes.len() {
            let crossings = subsistence::crossing_candidates(world, &world.tribes[index]);
            let tribe = &mut world.tribes[index];
            let here = tribe.cell;
            let n = tribe.headcount().max(1) as f32;
            let sites = &cultural_sites[tribe.culture as usize];
            let home = sites
                .iter()
                .chain(founded_sites.get(&tribe.id).into_iter().flatten())
                .min_by(|a, b| {
                    world
                        .grid
                        .angular_distance(here, a.cell)
                        .total_cmp(&world.grid.angular_distance(here, b.cell))
                });
            let secure =
                tribe.food / n >= SOCIAL.scarce_food_days && tribe.health >= SOCIAL.scarce_health;
            let score = |id: CellId| {
                let c = &world.cells[id as usize];
                if c.biome.water() {
                    return f32::NEG_INFINITY;
                }
                let herds: f32 = self
                    .herds
                    .at(id)
                    .iter()
                    .map(|&(_, i)| &world.herds[i])
                    .filter(|h| !h.species.small())
                    .map(|h| h.animals as f32 * h.species.meat() / 40.)
                    .sum();
                let crowd = self
                    .tribes
                    .at(id)
                    .iter()
                    .filter(|&&(_, i)| i != index)
                    .map(|&(_, i)| populations[i] as f32)
                    .sum::<f32>()
                    / n;
                let r = &c.resources;
                let aquatic: f32 = std::iter::once(id)
                    .chain(world.grid.neighbors(id))
                    .map(|site| &world.cells[site as usize])
                    .filter(|c| {
                        c.biome != Biome::Ocean
                            || (tribe.equipment.boat_places >= n * 0.25
                                && tribe.knowledge.level(Skill::Watercraft) >= 0.2)
                    })
                    .map(|c| c.resources.aquatic.total() * (1. - c.weather.snow * 0.9))
                    .sum();
                let food = (r.plants
                    + r.fruit
                    + r.game * 0.5
                    + aquatic * tribe.knowledge.level(Skill::Fishing))
                    / (n * (1. + crowd * 0.6));
                let attachment = if secure {
                    home.map_or(0., |site| {
                        let progress = (world.grid.angular_distance(here, site.cell)
                            - world.grid.angular_distance(id, site.cell))
                            * radius
                            / world.config.cell_km;
                        site.condition
                            * (progress * SOCIAL.seasonal_return_score
                                + if site.cell == id {
                                    SOCIAL.site_loyalty_score
                                } else {
                                    0.
                                })
                    })
                } else {
                    0.
                };
                attachment
                    + food.min(25.) / (1. + r.foraging_pressure)
                    + (herds / n * 6.).min(18.)
                    + (r.grazing / 4000.).min(3.)
                    - c.weather.snow * 10.
                    - (c.weather.temperature - 18.).abs() * 0.18
                    + if c.river > 0. { 2. } else { 0. }
            };
            tribe.travel = (tribe.travel
                + 14.
                    * (0.4 + tribe.health * 0.6)
                    * (1. - world.cells[here as usize].weather.snow * 0.6))
                // Enough credit to cross even two swamp cells; difficult terrain
                // delays a move rather than becoming permanently impassable.
                .min(world.config.cell_km * 4.);
            if tribe.travel < world.config.cell_km * 0.5 {
                continue;
            }
            let mut best = here;
            let mut best_cost = 0.;
            let mut by_water = false;
            let mut best_score = score(here) + 2.;
            for next in world.grid.neighbors(here) {
                let river_route = world.cells[here as usize].river > 0.
                    && world.cells[next as usize].river > 0.
                    && (world.cells[here as usize].drainage == Some(next)
                        || world.cells[next as usize].drainage == Some(here))
                    && tribe.equipment.boat_places >= n
                    && tribe.knowledge.level(Skill::Watercraft) > 0.2
                    && world.cells[here as usize].weather.snow < 0.5
                    && world.cells[here as usize].weather.precipitation < 4.;
                let cost = world.grid.angular_distance(here, next)
                    * radius
                    * if river_route {
                        0.65
                    } else {
                        (world.cells[here as usize].biome.travel_cost()
                            + world.cells[next as usize].biome.travel_cost())
                            * 0.5
                    };
                if cost > tribe.travel {
                    continue;
                }
                let s = score(next) + world.rng.unit() * 1.5;
                if s > best_score {
                    best = next;
                    best_score = s;
                    best_cost = cost;
                    by_water = river_route;
                }
            }
            for (next, cost) in crossings {
                let s = score(next) - 2. + world.rng.unit();
                if s > best_score {
                    best = next;
                    best_score = s;
                    best_cost = cost;
                    by_water = true;
                }
            }
            if best != here {
                tribe.cell = best;
                tribe.residence_days = 0;
                tribe.travel -= best_cost;
                world.totals.migrations += 1;
                if by_water {
                    world.totals.water_crossings += 1;
                    tribe.equipment.boat_places *= 0.96;
                    // Bad luck costs provisions and health; ensuing mortality
                    // remains in the existing health/nutrition accounting.
                    if world
                        .rng
                        .chance(0.12 * (1. - tribe.knowledge.level(Skill::Navigation)))
                    {
                        tribe.food *= 0.8;
                        tribe.health_shock(0.95);
                    }
                }
                events.push(Event {
                    day: world.day,
                    kind: if by_water {
                        EventKind::WaterCrossing
                    } else {
                        EventKind::Migration
                    },
                    cell: best,
                    tribe: tribe.id,
                    other: None,
                    amount: tribe.headcount(),
                    probability: 1.,
                });
            }
        }
        for e in events {
            world.event(e);
        }
    }

    fn feed_tribes(&self, world: &mut World) {
        let mut events = Vec::new();
        for t in &mut world.tribes {
            let n = t.headcount() as f32;
            if n == 0. {
                continue;
            }
            // Choose obtainable returns, not the first herd in the spatial bucket.
            let mut trap_target = None;
            let mut hunt_target = None;
            let mut trap_score = 0.;
            let mut hunt_score = 0.;
            for site in std::iter::once(t.cell).chain(
                world
                    .grid
                    .neighbors(t.cell)
                    .filter(|_| world.config.cell_km <= 30.),
            ) {
                let habitat = &world.cells[site as usize];
                let area = (world.config.cell_km / 20.).powi(2) * world.grid.area_factor(site);
                let cover = ((if habitat.biome.woodland() { 0.75 } else { 0.15 })
                    + (habitat.elevation.max(0) as f32 / 5000.).min(0.2))
                .min(0.95);
                let distance = world.grid.angular_distance(t.cell, site) * world.config.radius_km();
                for &(_, i) in self.herds.at(site) {
                    let herd = &world.herds[i];
                    if herd.animals == 0 {
                        continue;
                    }
                    let encounter = humanity_fauna::hunting::encounter(herd, area, cover, distance);
                    let score = encounter * herd.species.meat()
                        / (herd.species.traits().handling + distance * 0.1);
                    if herd.species.small() && site == t.cell && score > trap_score {
                        trap_target = Some((i, encounter));
                        trap_score = score;
                    } else if !herd.species.small() && score > hunt_score {
                        hunt_target = Some((i, encounter, habitat.weather.snow));
                        hunt_score = score;
                    }
                }
            }
            let cell = &mut world.cells[t.cell as usize];
            let access = (1. - cell.weather.snow * 0.8) / (1. + cell.resources.foraging_pressure)
                * if t.residence_days == 0 {
                    FORAGING.moving_work_fraction
                } else {
                    1.
                };
            let effort = 0.9;
            let hunters = t.labor(Role::Hunter) * 0.6;
            let work = t.labor(Role::Gatherer)
                * 3.2
                * access
                * effort
                * (0.75 + t.role_skill(Role::Gatherer, Skill::PlantProcessing) * 0.7);
            let area = (world.config.cell_km / 20.).powi(2) * world.grid.area_factor(t.cell);
            // Searching unsuccessfully still consumes labor. Regional biomass is
            // not a pile at camp, and depleted patches yield less before exhaustion.
            let gathered = foraging::gather(&mut cell.resources, work, area);
            t.food += gathered;
            world.totals.gathered += f64::from(gathered);
            let moving = if t.residence_days == 0 {
                FORAGING.moving_work_fraction
            } else {
                1.
            };
            if work > 0. {
                t.activity(
                    world.day,
                    Role::Gatherer,
                    Skill::PlantProcessing,
                    moving * 0.9,
                    gathered > 0.,
                );
            }
            if hunters > 0.
                && t.food < n * 12.
                && let Some((i, encounter)) = trap_target
            {
                let herd = &mut world.herds[i];
                let effort = if hunt_target.is_some() { 0.3 } else { 0.9 };
                let (kills, probability) = herd.trap(
                    t.labor(Role::Hunter) * effort * moving,
                    t.role_skill(Role::Hunter, Skill::Trapping),
                    encounter,
                    &mut world.rng,
                    &mut world.fauna_stats,
                );
                let food = kills as f32 * herd.species.meat();
                t.food += food;
                cell.resources.game = (cell.resources.game - food).max(0.);
                world.totals.animals_hunted += u64::from(kills);
                if kills > 0 {
                    world.totals.hunts += 1;
                } else {
                    world.totals.failed_hunts += 1;
                }
                t.activity(
                    world.day,
                    Role::Hunter,
                    Skill::Trapping,
                    moving * effort,
                    kills > 0,
                );
                t.knowledge.prey[herd.species.index()] = (t.knowledge.prey[herd.species.index()]
                    + if kills > 0 { 0.01 } else { 0.002 })
                .min(1.);
                events.push(Event {
                    day: world.day,
                    kind: if kills > 0 {
                        EventKind::Hunt
                    } else {
                        EventKind::HuntFailed
                    },
                    cell: herd.cell,
                    tribe: t.id,
                    other: Some(herd.id),
                    amount: kills,
                    probability,
                });
            }
            if t.food < n * 12.
                && t.specialists.hunters > 0
                && let Some((i, encounter, snow)) = hunt_target
            {
                let herd = &mut world.herds[i];
                let p = hunting::success_probability(hunting::Hunt {
                    tracking: t.role_skill(Role::Hunter, Skill::Tracking),
                    cooperative: t.role_skill(Role::Hunter, Skill::DriveHunting),
                    hunters: hunters * moving,
                    tool_skill: t.role_skill(Role::Hunter, Skill::Knapping),
                    tools: t.equipment.stone / n,
                    familiarity: t.knowledge.prey[herd.species.index()],
                    difficulty: herd.species.difficulty(),
                    snow,
                    distant: herd.cell != t.cell,
                }) * encounter
                    * (hunters * moving / herd.species.traits().handling).min(1.);
                let success = world.rng.chance(p);
                let kills = if success {
                    world
                        .rng
                        .expected((hunters * moving / herd.species.traits().handling).max(1.))
                        .min(((n * 12. - t.food) / herd.species.meat()).ceil() as u32)
                        .min(herd.animals)
                } else {
                    0
                };
                let kills = herd.harvest(kills, &mut world.fauna_stats);
                t.food += kills as f32 * herd.species.meat();
                t.equipment.hides = (t.equipment.hides + kills as f32 * 2.).min(n * 2.);
                let familiarity = &mut t.knowledge.prey[herd.species.index()];
                *familiarity = (*familiarity + if success { 0.015 } else { 0.003 }).min(1.);
                t.activity(
                    world.day,
                    Role::Hunter,
                    Skill::Tracking,
                    moving * 0.4,
                    success,
                );
                t.activity(
                    world.day,
                    Role::Hunter,
                    Skill::DriveHunting,
                    moving * 0.2,
                    success,
                );
                if success {
                    world.totals.hunts += 1;
                    world.totals.animals_hunted += u64::from(kills);
                } else {
                    world.totals.failed_hunts += 1;
                }
                events.push(Event {
                    day: world.day,
                    kind: if success {
                        EventKind::Hunt
                    } else {
                        EventKind::HuntFailed
                    },
                    cell: herd.cell,
                    tribe: t.id,
                    other: Some(herd.id),
                    amount: kills,
                    probability: p,
                });
            }
            let clothing = t.knowledge.level(Skill::Clothing) * (t.equipment.hides / n).min(1.);
            let need = t
                .people
                .members
                .iter()
                .map(|h| {
                    consumption::ration(
                        h.age(world.day, world.config.year_days()),
                        cell.weather.temperature,
                        clothing,
                    )
                })
                .sum();
            let preservation = t.knowledge.level(Skill::Preservation)
                * (0.4 + 0.6 * (t.equipment.wood / (n * 0.02).max(1.)).min(1.));
            t.equipment.wood = (t.equipment.wood - n * preservation * 0.002).max(0.);
            let nutrition = consumption::consume(
                &mut t.food,
                need,
                preservation,
                n * (20. + t.knowledge.level(Skill::Cordage) * 5. + preservation * 20.),
            );
            let care = t.labor(Role::Healer) / n
                * (0.08 + t.role_skill(Role::Healer, Skill::Healing) * 0.08);
            for h in &mut t.people.members {
                h.health = consumption::health_after(h.health, nutrition, care);
            }
            if care > 0. {
                t.activity(world.day, Role::Healer, Skill::Healing, moving * 0.9, true);
            }
            t.refresh(world.day);
        }
        for e in events {
            world.event(e);
        }
    }

    fn contacts(&self, world: &mut World) {
        let mut events = Vec::new();
        for a in 0..world.tribes.len() {
            if world.tribes[a].headcount() == 0 {
                continue;
            }
            let cell = world.tribes[a].cell;
            // Bounded samples from local buckets; dense populations don't become quadratic.
            let positions = std::iter::once(cell).chain(world.grid.neighbors(cell));
            let mut partners = [usize::MAX; 7];
            for (slot, pos) in positions.enumerate() {
                let bucket = self.tribes.at(pos);
                if !bucket.is_empty() {
                    partners[slot] = bucket[world.rng.index(bucket.len())].1;
                }
            }
            for b in partners {
                if b <= a || b >= world.tribes.len() || world.tribes[b].headcount() == 0 {
                    continue;
                }
                if !world.rng.chance(if world.tribes[b].cell == cell {
                    0.12
                } else {
                    0.025
                }) {
                    continue;
                }
                let (left, right) = world.tribes.split_at_mut(b);
                let (ta, tb) = (&mut left[a], &mut right[0]);
                let language = understanding(&ta.languages, &tb.languages);
                let scarcity = (1.
                    - (ta.food / ta.headcount() as f32).min(tb.food / tb.headcount() as f32) / 5.)
                    .clamp(0., 1.);
                let p = ((1. - language) * 0.18 + scarcity * 0.5).clamp(0.02, 0.8);
                if world.rng.chance(p) {
                    let strength_a =
                        ta.specialists.hunters as f32 * ta.health * (0.5 + ta.hunting_skill);
                    let strength_b =
                        tb.specialists.hunters as f32 * tb.health * (0.5 + tb.hunting_skill);
                    let win = strength_a / (strength_a + strength_b).max(1.);
                    let a_wins = world.rng.chance(win);
                    let (winner, loser) = if a_wins {
                        (&mut *ta, &mut *tb)
                    } else {
                        (&mut *tb, &mut *ta)
                    };
                    let casualty_rate = 0.01 + world.rng.unit() * 0.06;
                    let casualties = world.rng.expected(loser.headcount() as f32 * casualty_rate);
                    let deaths = loser.people.remove(casualties, &mut world.rng);
                    let loot = loser.food * 0.3;
                    loser.food -= loot;
                    winner.food += loot;
                    loser.health_shock(0.92);
                    world.totals.conflicts += 1;
                    world.totals.deaths += u64::from(deaths);
                    world.insights.death(
                        world.day,
                        loser,
                        &world.cells[loser.cell as usize],
                        DeathCause::Conflict,
                        deaths,
                    );
                    events.push(Event {
                        day: world.day,
                        kind: EventKind::Conflict,
                        cell,
                        tribe: winner.id,
                        other: Some(loser.id),
                        amount: deaths,
                        probability: if a_wins { win } else { 1. - win },
                    });
                } else {
                    social::friendly_contact(ta, tb, world.day);
                    let a_language = dominant(&ta.languages);
                    if let Some(skill) = subsistence::exchange(ta, tb, language, &mut world.rng) {
                        world.totals.knowledge_exchanges += 1;
                        events.push(Event {
                            day: world.day,
                            kind: EventKind::KnowledgeExchange,
                            cell,
                            tribe: ta.id,
                            other: Some(tb.id),
                            amount: skill as u32,
                            probability: subsistence::transmission_probability(language),
                        });
                    }
                    let b_language = dominant(&tb.languages);
                    learn(&mut ta.languages, b_language);
                    learn(&mut tb.languages, a_language);
                    subsistence::trade_materials(ta, tb);
                    let a_days = ta.food / ta.headcount() as f32;
                    let b_days = tb.food / tb.headcount() as f32;
                    let (giver, receiver) = if a_days > b_days {
                        (&mut *ta, &mut *tb)
                    } else {
                        (&mut *tb, &mut *ta)
                    };
                    let food = (giver.food - giver.headcount() as f32 * 3.).max(0.) * 0.1;
                    giver.food -= food;
                    receiver.food += food;
                    world.totals.exchanges += 1;
                    events.push(Event {
                        day: world.day,
                        kind: EventKind::Exchange,
                        cell,
                        tribe: ta.id,
                        other: Some(tb.id),
                        amount: food as u32,
                        probability: 1. - p,
                    });
                }
            }
        }
        for e in events {
            world.event(e);
        }
    }
}

fn dominant(languages: &[Language]) -> u32 {
    languages
        .iter()
        .max_by(|a, b| a.share.total_cmp(&b.share))
        .map_or(0, |l| l.family)
}
fn learn(languages: &mut Vec<Language>, family: u32) {
    for l in languages.iter_mut() {
        l.share *= 0.97;
    }
    if let Some(l) = languages.iter_mut().find(|l| l.family == family) {
        l.share += 0.03;
    } else {
        languages.push(Language {
            family,
            share: 0.03,
        });
    }
    languages.sort_by(|a, b| b.share.total_cmp(&a.share).then(a.family.cmp(&b.family)));
    languages.truncate(4);
    let sum: f32 = languages.iter().map(|l| l.share).sum();
    for l in languages {
        l.share /= sum;
    }
}

fn demography(world: &mut World) {
    let mut events = Vec::new();
    let year = world.config.year_days() as f32;
    for t in &mut world.tribes {
        let before = t.headcount();
        if before == 0 {
            continue;
        }
        let birth_p = 0.16 * t.health.powi(2) / year;
        let p = 0.018 / year + (1. - t.health).powi(3) * 0.025;
        let changes = humanity_core::demography::advance(
            &mut t.people,
            world.day,
            t.id,
            &mut world.next_human_id,
            &mut world.rng,
        );
        let births = changes.births;
        let deaths = changes.deaths();
        for (cause, count) in [
            (DeathCause::Background, changes.background),
            (DeathCause::Nutrition, changes.nutrition),
            (DeathCause::Aging, changes.aging),
        ] {
            world
                .insights
                .death(world.day, t, &world.cells[t.cell as usize], cause, count);
        }
        t.refresh(world.day);
        world.totals.births += u64::from(births);
        world.totals.deaths += u64::from(deaths);
        for (amount, kind, probability) in [
            (births, EventKind::Birth, birth_p),
            (deaths, EventKind::Death, p),
        ] {
            if amount > 0 {
                events.push(Event {
                    day: world.day,
                    kind,
                    cell: t.cell,
                    tribe: t.id,
                    other: None,
                    amount,
                    probability,
                });
            }
        }
        if t.headcount() == 0 {
            events.push(Event {
                day: world.day,
                kind: EventKind::Extinction,
                cell: t.cell,
                tribe: t.id,
                other: None,
                amount: before,
                probability: 1.,
            });
        }
    }
    for e in events {
        world.event(e);
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    fn world() -> World {
        humanity_world::generate(PlanetConfig {
            resolution: 12,
            cell_km: 20.,
            temperature: 28.,
            season_days: 30,
            axial_tilt: 23.4,
            ocean_fraction: 0.6,
            rainfall: 1.,
            population: 3000,
            seed: 42,
        })
        .unwrap()
    }
    #[test]
    fn replay_and_population_accounting_survive_a_year() {
        let mut a = world();
        let mut b = a.clone();
        Simulation::default().advance(&mut a, 120);
        let mut sim = Simulation::default();
        sim.advance(&mut b, 55);
        let json = serde_json::to_string(&b).unwrap();
        b = serde_json::from_str(&json).unwrap();
        Simulation::default().advance(&mut b, 65);
        assert_eq!(
            serde_json::to_value(&a).unwrap(),
            serde_json::to_value(&b).unwrap()
        );
        assert_eq!(a.population(), 3000 + a.totals.births - a.totals.deaths);
        assert!(a.totals.hunts > 0 && a.totals.migrations > 0 && a.totals.exchanges > 0);
        assert!(a.events.len() <= 512);
        for t in &a.tribes {
            assert!(!a.cells[t.cell as usize].biome.water());
            assert!(t.food.is_finite() && t.food >= 0.);
            assert!((t.languages.iter().map(|l| l.share).sum::<f32>() - 1.).abs() < 0.001);
        }
    }
}
