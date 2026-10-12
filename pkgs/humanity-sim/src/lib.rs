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
        if world.insights.since_day.is_none() {
            world.record_population();
        }
        world.day += 1;
        humanity_world::update_environment(world, false);
        migrate_herds(world);
        self.herds.rebuild(world.herds.iter().map(|h| h.cell));
        self.tribes.rebuild(world.tribes.iter().map(|t| t.cell));
        subsistence::daily(world);
        self.move_tribes(world);
        self.feed_tribes(world);
        self.tribes.rebuild(world.tribes.iter().map(|t| t.cell));
        self.contacts(world);
        demography(world);
        social::daily(world);
        social::fission(world);
        world.herds.retain(|h| h.animals > 0);
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
                || site.season == world.local_season(site.cell)
            {
                cultural_sites[site.culture as usize].push(site);
                founded_sites
                    .entry(site.founder_band)
                    .or_default()
                    .push(site);
            }
        }
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
                let herds: u32 = self
                    .herds
                    .at(id)
                    .iter()
                    .map(|&(_, i)| world.herds[i].animals)
                    .sum();
                let crowd = self.tribes.at(id).len() as f32;
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
                        progress * SOCIAL.seasonal_return_score
                            + if site.cell == id {
                                SOCIAL.site_loyalty_score
                            } else {
                                0.
                            }
                    })
                } else {
                    0.
                };
                attachment
                    + food.min(25.)
                    + (herds as f32 / n * 6.).min(18.)
                    + (r.grazing / 4000.).min(3.)
                    - c.weather.snow * 10.
                    - (c.weather.temperature - 18.).abs() * 0.18
                    + if c.river > 0. { 2. } else { 0. }
            };
            tribe.travel = (tribe.travel
                + 14.
                    * (0.4 + tribe.health * 0.6)
                    * (1. - world.cells[here as usize].weather.snow * 0.6))
                .min(world.config.cell_km * 2.);
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
                    * if river_route { 0.65 } else { 1. };
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
                        tribe.health *= 0.95;
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
            let cell = &mut world.cells[t.cell as usize];
            let access = (1. - cell.weather.snow * 0.8) * (0.35 + 0.65 * t.health);
            let effort = 0.9 * (1. - t.fishing_effort);
            let hunters = t.specialists.hunters as f32 * effort;
            let work = t.specialists.gatherers as f32
                * 3.2
                * access
                * effort
                * (0.75 + t.knowledge.level(Skill::PlantProcessing) * 0.7);
            let fruit = cell.resources.fruit.min(work * 0.55);
            let plants = cell.resources.plants.min(work - fruit);
            let game = cell
                .resources
                .game
                .min(hunters * (0.5 + t.knowledge.level(Skill::Trapping)) * access);
            cell.resources.fruit -= fruit;
            cell.resources.plants -= plants;
            cell.resources.game -= game;
            t.food += fruit + plants + game;
            world.totals.gathered += f64::from(fruit + plants + game);
            if t.food < n * 12. && t.specialists.hunters > 0 {
                let mut target = self
                    .herds
                    .at(t.cell)
                    .iter()
                    .map(|&(_, i)| i)
                    .find(|&i| world.herds[i].animals > 0);
                if target.is_none() && world.config.cell_km <= 30. {
                    for neighbor in world.grid.neighbors(t.cell) {
                        target = self
                            .herds
                            .at(neighbor)
                            .iter()
                            .map(|&(_, i)| i)
                            .find(|&i| world.herds[i].animals > 0);
                        if target.is_some() {
                            break;
                        }
                    }
                }
                if let Some(i) = target {
                    let herd = &mut world.herds[i];
                    let p = (0.12
                        + t.hunting_skill * 0.3
                        + hunters * 0.003
                        + t.knowledge.level(Skill::Tracking) * 0.15
                        + t.knowledge.level(Skill::DriveHunting) * 0.15
                        + t.knowledge.level(Skill::Knapping)
                            * (t.equipment.stone / n).min(1.)
                            * 0.1
                        + t.knowledge.prey[herd.species.index()] * 0.25
                        - herd.species.difficulty()
                        - cell.weather.snow * 0.15
                        - if herd.cell != t.cell { 0.15 } else { 0. })
                    .clamp(0.05, 0.9);
                    let success = world.rng.chance(p);
                    let kills = if success {
                        (1 + world.rng.index((hunters as u32 / 5).max(1) as usize) as u32)
                            .min(herd.animals)
                    } else {
                        0
                    };
                    herd.animals -= kills;
                    t.food += kills as f32 * herd.species.meat();
                    t.equipment.hides = (t.equipment.hides + kills as f32 * 2.).min(n * 2.);
                    let familiarity = &mut t.knowledge.prey[herd.species.index()];
                    *familiarity = (*familiarity + if success { 0.015 } else { 0.003 }).min(1.);
                    t.hunting_skill =
                        (t.hunting_skill + if success { 0.002 } else { 0.0005 }).min(1.);
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
            }
            let clothing = t.knowledge.level(Skill::Clothing) * (t.equipment.hides / n).min(1.);
            let need = n
                * 0.9
                * (1. + (-cell.weather.temperature).max(0.) * 0.006 * (1. - clothing * 0.7));
            let eaten = t.food.min(need);
            let preservation = t.knowledge.level(Skill::Preservation)
                * (0.4 + 0.6 * (t.equipment.wood / (n * 0.02).max(1.)).min(1.));
            t.equipment.wood = (t.equipment.wood - n * preservation * 0.002).max(0.);
            t.food = ((t.food - eaten) * (0.97 + preservation * 0.022))
                .min(n * (20. + t.knowledge.level(Skill::Cordage) * 5. + preservation * 20.));
            let nutrition = eaten / need;
            let healing = 0.008
                + t.specialists.healers as f32 / n
                    * (0.08 + t.knowledge.level(Skill::Healing) * 0.08);
            t.health = (t.health
                + if nutrition > 0.95 {
                    healing
                } else {
                    -(1. - nutrition) * 0.065
                })
            .clamp(0., 1.);
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
                    loser.health *= 0.92;
                    loser.specialists = Specialists::from_adults(loser.people.adults());
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

fn migrate_herds(world: &mut World) {
    let radius = world.config.radius_km();
    for h in &mut world.herds {
        if h.animals == 0 {
            continue;
        }
        let ideal = h.species.ideal_temperature();
        let score = |id: CellId| {
            let c = &world.cells[id as usize];
            if c.biome.water() {
                return f32::NEG_INFINITY;
            }
            (c.resources.grazing / h.animals as f32).min(70.) + h.species.habitat(c)
                - (c.weather.temperature - ideal).abs() * 1.5
                - c.weather.snow * 35.
        };
        let here = h.cell;
        h.travel = (h.travel + 18. * (1. - world.cells[here as usize].weather.snow * 0.5))
            .min(world.config.cell_km * 2.);
        if h.travel >= world.config.cell_km * 0.5 {
            let mut best = here;
            let mut best_cost = 0.;
            let mut best_score = score(here) + 3.;
            for next in world.grid.neighbors(here) {
                let cost = world.grid.angular_distance(here, next) * radius;
                if cost > h.travel {
                    continue;
                }
                let s = score(next) + world.rng.unit() * 3.;
                if s > best_score {
                    best = next;
                    best_score = s;
                    best_cost = cost;
                }
            }
            if best != here {
                h.cell = best;
                h.travel -= best_cost;
            }
        }
        let c = &mut world.cells[h.cell as usize];
        let need = h.animals as f32 * h.species.grazing_need();
        let eaten = c.resources.grazing.min(need);
        c.resources.grazing -= eaten;
        h.condition = (h.condition + (eaten / need - 0.85) * 0.08).clamp(0., 1.);
        let annual = world.config.year_days() as f32;
        let breeding = if c.weather.temperature > ideal - 5. && c.weather.temperature < ideal + 12.
        {
            h.species.annual_breeding()
        } else {
            0.03
        };
        let births = world
            .rng
            .expected(h.animals as f32 * breeding * h.condition / annual);
        let deaths = world
            .rng
            .expected(h.animals as f32 * (0.09 / annual + (1. - h.condition).powi(3) * 0.025));
        h.animals = h.animals.saturating_add(births).saturating_sub(deaths);
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
        let births = world.rng.expected(t.people.female[1] as f32 * birth_p);
        let girls = world.rng.expected(births as f32 * 0.49).min(births);
        t.people.female[0] += girls;
        t.people.male[0] += births - girls;
        let children = t.people.female[0] + t.people.male[0];
        for k in &mut t.knowledge.practitioners {
            k[0] *= (children - births) as f32 / children.max(1) as f32;
        }
        let p = 0.018 / year + (1. - t.health).powi(3) * 0.025;
        let mortality = world.rng.expected(t.headcount() as f32 * p);
        let mut deaths = t.people.remove(mortality, &mut world.rng);
        // Competing hazards: classify the resolved deaths in proportion to their
        // hazards, using an isolated RNG so instrumentation never alters gameplay.
        let mut attribution =
            Rng::new(world.day.wrapping_mul(0x9e3779b97f4a7c15) ^ u64::from(t.id));
        let background = attribution
            .expected(deaths as f32 * (0.018 / year) / p)
            .min(deaths);
        let cell = &world.cells[t.cell as usize];
        world
            .insights
            .death(world.day, t, cell, DeathCause::Background, background);
        world.insights.death(
            world.day,
            t,
            cell,
            DeathCause::Nutrition,
            deaths - background,
        );
        let mut aging = 0;
        let previous_adults = t.people.adults();
        let previous_elders = t.people.female[2] + t.people.male[2];
        let mut grown_total = 0;
        let mut aged_total = 0;
        for cohorts in [&mut t.people.female, &mut t.people.male] {
            let old_deaths = world
                .rng
                .expected(cohorts[2] as f32 * 0.08 / year)
                .min(cohorts[2]);
            cohorts[2] -= old_deaths;
            deaths += old_deaths;
            aging += old_deaths;
            let grown = world
                .rng
                .expected(cohorts[0] as f32 / (15. * year))
                .min(cohorts[0]);
            let aged = world
                .rng
                .expected(cohorts[1] as f32 / (35. * year))
                .min(cohorts[1]);
            cohorts[0] -= grown;
            cohorts[1] = cohorts[1] + grown - aged;
            cohorts[2] += aged;
            grown_total += grown;
            aged_total += aged;
        }
        for k in &mut t.knowledge.practitioners {
            let old = *k;
            k[1] = (old[1] * (previous_adults - aged_total) as f32 + old[0] * grown_total as f32)
                / t.people.adults().max(1) as f32;
            let elders = t.people.female[2] + t.people.male[2];
            k[2] = (old[2] * (previous_elders - aging) as f32 + old[1] * aged_total as f32)
                / elders.max(1) as f32;
            if t.people.female[0] + t.people.male[0] == 0 {
                k[0] = 0.;
            }
            // Small practices can vanish with their last teachers. Large bands
            // use expected fractions rather than per-person entities.
            for (level, count) in k.iter_mut().zip([
                t.people.female[0] + t.people.male[0],
                t.people.adults(),
                elders,
            ]) {
                *level = level.clamp(0., 1.);
                if *level * (count as f32) < 1.
                    && deaths > 0
                    && world.rng.chance(deaths as f32 / before as f32)
                {
                    *level = 0.;
                }
            }
        }
        t.specialists = Specialists::from_adults(t.people.adults());
        world
            .insights
            .death(world.day, t, cell, DeathCause::Aging, aging);
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
