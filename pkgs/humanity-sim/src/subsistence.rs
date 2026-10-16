use humanity_core::*;

pub fn transmission_probability(language: f32) -> f32 {
    0.04 + 0.66 * language
}

pub fn trade_materials(a: &mut Tribe, b: &mut Tribe) {
    let na = a.headcount().max(1) as f32;
    let nb = b.headcount().max(1) as f32;
    for (x, y) in [
        (&mut a.equipment.wood, &mut b.equipment.wood),
        (&mut a.equipment.fiber, &mut b.equipment.fiber),
        (&mut a.equipment.stone, &mut b.equipment.stone),
        (&mut a.equipment.hides, &mut b.equipment.hides),
    ] {
        let difference = *x / na - *y / nb;
        let transfer = difference * na.min(nb) * 0.1;
        *x -= transfer;
        *y += transfer;
    }
}

/// One topic per meeting, bidirectional teaching; neither group loses knowledge.
pub fn exchange(a: &mut Tribe, b: &mut Tribe, language: f32, rng: &mut Rng) -> Option<Skill> {
    let skill = Skill::ALL[rng.index(Skill::ALL.len())];
    if !rng.chance(transmission_probability(language)) {
        return None;
    }
    let (teacher, learner) = if a.knowledge.level(skill) > b.knowledge.level(skill) {
        (a, b)
    } else {
        (b, a)
    };
    let teacher_member = teacher
        .people
        .members
        .iter()
        .max_by(|a, b| a.skills.level(skill).total_cmp(&b.skills.level(skill)))?;
    let mut learned = false;
    for h in &mut learner.people.members {
        learned |= h.skills.learn(
            skill,
            teacher_member.skills.level(skill),
            0.1 * (0.1 + 0.9 * language),
            true,
        ) > 0.;
    }
    if !learned {
        return None;
    }
    learner.knowledge = learner.people.knowledge(&learner.knowledge.prey);
    for (a, b) in learner
        .knowledge
        .prey
        .iter_mut()
        .zip(teacher.knowledge.prey.iter().copied())
    {
        *a += (b - *a).max(0.) * 0.08;
    }
    Some(skill)
}

fn opportunity(skill: Skill, c: &Cell, water: bool) -> bool {
    match skill {
        Skill::Fishing | Skill::Watercraft | Skill::Navigation => water,
        Skill::Cordage | Skill::Trapping => c.resources.fiber > 1.,
        Skill::Knapping => c.resources.toolstone > 0.15,
        Skill::Clothing => c.weather.temperature < 15.,
        _ => !c.biome.water(),
    }
}

pub fn daily(world: &mut World) {
    let mut events = Vec::new();
    let mut wildlife = vec![Vec::new(); world.cells.len()];
    for group in &world.herds {
        if group.animals > 0 {
            wildlife[group.cell as usize].push(group);
        }
    }
    for t in &mut world.tribes {
        let n = t.headcount() as f32;
        if n == 0. {
            continue;
        }
        let sites: Vec<_> = std::iter::once(t.cell)
            .chain(world.grid.neighbors(t.cell))
            .collect();
        let water = sites
            .iter()
            .any(|&id| world.cells[id as usize].resources.aquatic.total() > 1.);
        let home = &world.cells[t.cell as usize];
        let area = (world.config.cell_km / 20.).powi(2) * world.grid.area_factor(t.cell);
        let access = (1. - home.weather.snow * 0.8) / (1. + home.resources.foraging_pressure);
        let aquatic_opportunity = sites
            .iter()
            .map(|&id| {
                let site = &world.cells[id as usize];
                if site.biome == Biome::Ocean && t.equipment.boat_places < n * 0.25 {
                    return 0.;
                }
                let distance = world.grid.angular_distance(t.cell, id) * world.config.radius_km();
                fishing::return_rate(
                    site.resources.aquatic.total(),
                    1.,
                    area,
                    distance,
                    site.weather.snow,
                    site.weather.precipitation,
                    site.resources.foraging_pressure,
                ) * 4.
            })
            .fold(0_f32, f32::max);
        let mut large_returns = 0_f32;
        let mut small_returns = 0_f32;
        for &id in &sites {
            if id != t.cell && world.config.cell_km > 30. {
                continue;
            }
            let c = &world.cells[id as usize];
            let cover = ((if c.biome.woodland() { 0.75 } else { 0.15 })
                + (c.elevation.max(0) as f32 / 5000.).min(0.2))
            .min(0.95);
            let area = (world.config.cell_km / 20.).powi(2) * world.grid.area_factor(id);
            let distance = world.grid.angular_distance(t.cell, id) * world.config.radius_km();
            for h in &wildlife[id as usize] {
                let encounter = humanity_fauna::hunting::encounter(h, area, cover, distance);
                let returns = encounter * h.species.meat() / h.species.traits().handling
                    * (1. - c.weather.snow * 0.5);
                if h.species.small() && id == t.cell {
                    small_returns = small_returns.max(returns * 0.5);
                } else if !h.species.small() {
                    large_returns = large_returns.max(returns * 0.4);
                }
            }
        }
        t.assign_roles(
            world.day,
            Opportunities {
                gathering: 3.2
                    * access
                    * encounter(
                        home.resources.plants + home.resources.fruit,
                        area,
                        FORAGING.plant_half_density,
                    ),
                hunting: if large_returns > 0. {
                    large_returns * 0.6 + small_returns * 0.3
                } else {
                    small_returns * 0.9
                },
                fishing: aquatic_opportunity,
            },
        );
        t.teach_workers(world.day);
        let c = &mut world.cells[t.cell as usize];
        // A bounded experiment, not a universal technology clock.
        if world.day.is_multiple_of(7) && t.people.adults() > 0 {
            let skill = Skill::ALL[world.rng.index(Skill::ALL.len())];
            let p = 0.08 * t.health * (0.3 + t.knowledge.level(Skill::OralTradition));
            if t.knowledge.level(skill) < 0.08
                && opportunity(skill, c, water)
                && t.knowledge.prerequisites(skill)
                && world.rng.chance(p)
            {
                if let Some(h) = t.people.members.iter_mut().find(|h| {
                    h.can_work(world.day, world.config.year_days()) && h.skills.prerequisites(skill)
                }) {
                    h.skills.levels[skill as usize] = (h.skills.level(skill) + 0.03).min(1.);
                }
                t.knowledge = t.people.knowledge(&t.knowledge.prey);
                world.totals.innovations += 1;
                events.push(Event {
                    day: world.day,
                    kind: EventKind::Innovation,
                    cell: t.cell,
                    tribe: t.id,
                    other: None,
                    amount: skill as u32,
                    probability: p,
                });
            }
        }
        let fishing_labor = t.labor(Role::Fisher);
        let fishing = t.role_skill(Role::Fisher, Skill::Fishing);
        let guide_labor = t.labor(Role::Guide);
        let e = &mut t.equipment;
        let labor = t
            .people
            .members
            .iter()
            .filter(|h| h.role != Role::Dependent)
            .map(|h| h.health * h.work_fraction)
            .sum::<f32>()
            * if t.residence_days == 0 {
                FORAGING.moving_work_fraction
            } else {
                1.
            };
        // Everyone reserves a tenth of their day; designated crafters use the rest too.
        let moving = if t.residence_days == 0 {
            FORAGING.moving_work_fraction
        } else {
            1.
        };
        let craft_labor = labor * 0.1 + guide_labor * moving * 0.9;
        let wood = c
            .resources
            .wood
            .min(craft_labor * 0.4)
            .min((n * 2. - e.wood).max(0.));
        let fiber = c
            .resources
            .fiber
            .min(craft_labor * 0.4)
            .min((n - e.fiber).max(0.));
        c.resources.wood -= wood;
        e.wood += wood;
        c.resources.fiber -= fiber;
        e.fiber += fiber;
        let toolstone = c.resources.toolstone;
        e.stone = (e.stone + toolstone * craft_labor * 0.2).min(n);
        e.stone *= 0.998;
        e.hides *= 0.999;
        e.boat_places *= 0.999;
        let craft = t.knowledge.level(Skill::Watercraft);
        let mut built_places = 0.;
        if water && craft > 0.15 && t.knowledge.level(Skill::Cordage) > 0.1 {
            let places = (craft_labor * craft * 0.8)
                .min(e.wood / 2.)
                .min(e.fiber)
                .min((n * 1.15 - e.boat_places).max(0.));
            e.wood -= places * 2.;
            e.fiber -= places;
            e.boat_places += places;
            built_places = places;
        }
        let gear = t.knowledge.level(Skill::Cordage) * (e.fiber / (n * 0.1).max(1.)).min(1.);
        // Choose one daily fishing patch. Failed search and travel consume time,
        // rather than letting unused catch capacity sweep every neighboring stock.
        let mut best = None;
        let mut best_rate = 0.;
        for id in sites {
            let site = &world.cells[id as usize];
            if site.biome == Biome::Ocean && (e.boat_places < n * 0.25 || craft < 0.2) {
                continue;
            }
            let distance = world.grid.angular_distance(t.cell, id) * world.config.radius_km();
            let area = (world.config.cell_km / 20.).powi(2) * world.grid.area_factor(id);
            for (kind, (stock, efficiency)) in [
                (site.resources.aquatic.shellfish, 0.8),
                (site.resources.aquatic.freshwater_fish, fishing),
                (site.resources.aquatic.coastal_fish, fishing),
            ]
            .into_iter()
            .enumerate()
            {
                let rate = fishing::return_rate(
                    stock,
                    efficiency,
                    area,
                    distance,
                    site.weather.snow,
                    site.weather.precipitation,
                    site.resources.foraging_pressure,
                );
                if rate > best_rate {
                    best_rate = rate;
                    best = Some((id, kind));
                }
            }
        }
        let budget = fishing_labor * moving * 0.9 * (1. + fishing * 5. + gear * 2.);
        let catch = if let Some((id, kind)) = best {
            let aquatic = &mut world.cells[id as usize].resources.aquatic;
            let stock = match kind {
                0 => &mut aquatic.shellfish,
                1 => &mut aquatic.freshwater_fish,
                _ => &mut aquatic.coastal_fish,
            };
            let take = fishing::catch(stock, budget, best_rate, world.rng.unit());
            let area = (world.config.cell_km / 20.).powi(2) * world.grid.area_factor(id);
            world.cells[id as usize].resources.foraging_pressure +=
                take / (area * FORAGING.patch_harvest_scale);
            take
        } else {
            0.
        };
        e.fiber = (e.fiber - labor * t.fishing_effort * 0.004).max(0.);
        t.food += catch;
        if best.is_some() && fishing_labor > 0. {
            t.activity(
                world.day,
                Role::Fisher,
                Skill::Fishing,
                moving * 0.9,
                catch > 0.,
            );
        }
        // All active workers spend the reserved tenth of their day on real craft work.
        for h in &mut t.people.members {
            if h.role != Role::Dependent {
                let craft_time =
                    moving * h.work_fraction * if h.role == Role::Guide { 1. } else { 0.1 };
                if wood > 0. || fiber > 0. {
                    h.practice(
                        world.day,
                        if built_places > 0. {
                            Skill::Watercraft
                        } else {
                            Skill::Cordage
                        },
                        craft_time * 0.5,
                        true,
                    );
                }
                if toolstone > 0. {
                    h.practice(world.day, Skill::Knapping, craft_time * 0.5, true);
                }
            }
        }
        if wood > 0. || fiber > 0. {
            t.craft_stories(
                if built_places > 0. {
                    Skill::Watercraft
                } else {
                    Skill::Cordage
                },
                moving * 0.05,
            );
        }
        if toolstone > 0. {
            t.craft_stories(Skill::Knapping, moving * 0.05);
        }
        world.totals.fished += f64::from(catch);
        if catch > 1. && world.day.is_multiple_of(14) {
            events.push(Event {
                day: world.day,
                kind: EventKind::Fishing,
                cell: t.cell,
                tribe: t.id,
                other: None,
                amount: catch as u32,
                probability: 1.,
            });
        }
    }
    for e in events {
        world.event(e);
    }
}

/// Camps remain on land. Rafts/canoes open a two-edge crossing through one
/// water cell, bounded by physical range, passenger capacity, ice and weather.
pub fn crossing_candidates(world: &World, t: &Tribe) -> Vec<(CellId, f32)> {
    let craft = t.knowledge.level(Skill::Watercraft);
    if craft < 0.2 || t.equipment.boat_places < t.headcount() as f32 {
        return Vec::new();
    }
    let range = 20. + 100. * t.knowledge.level(Skill::Navigation);
    let mut result = Vec::new();
    for sea in world.grid.neighbors(t.cell) {
        let c = &world.cells[sea as usize];
        if !c.biome.water() || c.weather.snow > 0.5 || c.weather.precipitation > 4. {
            continue;
        }
        if c.biome == Biome::Ocean && t.knowledge.level(Skill::Navigation) < 0.6 {
            continue;
        }
        for land in world.grid.neighbors(sea) {
            if land == t.cell || world.cells[land as usize].biome.water() {
                continue;
            }
            let distance = (world.grid.angular_distance(t.cell, sea)
                + world.grid.angular_distance(sea, land))
                * world.config.radius_km();
            if distance <= range && distance <= t.travel {
                result.push((land, distance));
            }
        }
    }
    result
}

#[cfg(test)]
mod tests {
    use super::*;
    fn world() -> World {
        humanity_world::generate(PlanetConfig {
            resolution: 12,
            cell_km: 10.,
            temperature: 28.,
            season_days: 30,
            axial_tilt: 23.4,
            ocean_fraction: 0.6,
            rainfall: 1.,
            population: 600,
            seed: 42,
        })
        .unwrap()
    }

    #[test]
    fn elders_teach_children_and_adults_but_absent_knowledge_does_not_spread() {
        let mut t = world().tribes.remove(0);
        for h in &mut t.people.members {
            h.skills = Skills::default();
            h.skills.levels[Skill::Cordage as usize] = 0.5;
            if h.age(0, 120) >= 15 {
                h.role = Role::Fisher;
            }
        }
        let elder = t
            .people
            .members
            .iter_mut()
            .find(|h| h.age(0, 120) >= 50)
            .unwrap();
        elder.skills.levels[Skill::Fishing as usize] = 1.;
        t.teach_workers(0);
        t.activity(0, Role::Fisher, Skill::Fishing, 0.5, true);
        assert!(t.knowledge.level(Skill::Fishing) > 0.);
        assert!(t.knowledge.practitioners[Skill::Fishing as usize][0] > 0.);
        assert_eq!(t.knowledge.level(Skill::Watercraft), 0.);
    }

    #[test]
    fn material_exchange_conserves_portable_resources() {
        let mut a = world().tribes.remove(0);
        let mut b = a.clone();
        a.equipment.wood = 50.;
        b.equipment.wood = 2.;
        trade_materials(&mut a, &mut b);
        assert!((a.equipment.wood + b.equipment.wood - 52.).abs() < 1e-5);
        assert!(a.equipment.wood < 50. && b.equipment.wood > 2.);
    }

    #[test]
    fn actual_participants_gain_experience_listeners_only_bounded_story_knowledge() {
        let mut t = world().tribes.remove(0);
        for h in &mut t.people.members {
            h.skills = Skills::default();
            h.role = Role::Dependent;
        }
        t.people.members[0].role = Role::Hunter;
        t.people.members[0].skills.levels[Skill::Tracking as usize] = 0.6;
        let id = t.people.members[0].id;
        t.refresh(0);
        let before = t.knowledge.practitioners[Skill::Tracking as usize];
        t.activity(1, Role::Hunter, Skill::Tracking, 0.5, false);
        let hunter = t.people.members.iter().find(|h| h.id == id).unwrap();
        assert_eq!(hunter.skills.experience[Skill::Tracking as usize], 0.5);
        assert!(hunter.skills.level(Skill::Tracking) > 0.6);
        for listener in t.people.members.iter().filter(|h| h.id != id) {
            assert_eq!(listener.skills.experience[Skill::Tracking as usize], 0.);
            assert!(listener.skills.level(Skill::Tracking) > 0.);
            assert!(listener.skills.level(Skill::Tracking) <= 0.6 * LEARNING.story_ceiling);
        }
        assert_ne!(before, t.knowledge.practitioners[Skill::Tracking as usize]);
    }

    #[test]
    fn jobs_follow_opportunities_and_instruction_spends_both_workers_time() {
        let mut t = world().tribes.remove(0);
        t.equipment.stone = t.headcount() as f32;
        t.equipment.wood = t.headcount() as f32;
        t.equipment.boat_places = t.headcount() as f32 * 2.;
        t.assign_roles(
            0,
            Opportunities {
                gathering: 0.,
                hunting: 10.,
                fishing: 0.,
            },
        );
        assert_eq!(t.specialists.hunters, t.people.workers());
        assert_eq!(t.specialists.fishers, 0);
        for h in &mut t.people.members {
            h.skills.levels[Skill::Tracking as usize] = 0.;
        }
        let teacher = t
            .people
            .members
            .iter_mut()
            .find(|h| h.role == Role::Hunter)
            .unwrap();
        teacher.skills.levels[Skill::Tracking as usize] = 0.2;
        t.teach_workers(0);
        let time_spent: f32 = t.people.members.iter().map(|h| 1. - h.work_fraction).sum();
        assert!((time_spent - 2. * LEARNING.lesson_time).abs() < 0.0001);
        assert!(
            t.people
                .members
                .iter()
                .filter(|h| h.skills.level(Skill::Tracking) < 0.2)
                .all(|h| h.skills.level(Skill::Tracking) <= 0.2 * LEARNING.teacher_ceiling)
        );
        t.assign_roles(
            1,
            Opportunities {
                gathering: 0.,
                hunting: 0.,
                fishing: 10.,
            },
        );
        assert_eq!(t.specialists.fishers, t.people.workers());
        assert_eq!(t.specialists.hunters, 0);
    }

    #[test]
    fn shared_language_increases_actual_transmission_without_erasing_teacher() {
        let t = world().tribes.remove(0);
        let mut gains = [0.; 2];
        for (case, language) in [0., 1.].into_iter().enumerate() {
            for seed in 0..2000 {
                let mut a = t.clone();
                let mut b = t.clone();
                for s in Skill::ALL {
                    a.knowledge.seed(s, 0.9);
                    b.knowledge.seed(s, 0.2);
                }
                for h in &mut a.people.members {
                    h.skills.levels = [0.9; 13];
                }
                for h in &mut b.people.members {
                    h.skills.levels = [0.2; 13];
                }
                a.refresh(0);
                b.refresh(0);
                let before = a.knowledge.clone();
                if let Some(s) = exchange(&mut a, &mut b, language, &mut Rng::new(seed)) {
                    gains[case] += b.knowledge.level(s) - 0.2;
                }
                assert_eq!(a.knowledge.practitioners, before.practitioners);
            }
        }
        assert!(gains[0] > 0., "demonstration can bridge a language barrier");
        assert!(gains[1] > gains[0] * 5.);
    }

    #[test]
    fn aquatic_harvest_depletes_real_stocks_and_boats_require_materials() {
        let mut w = world();
        let cell = (0..w.cells.len() as u32)
            .find(|&id| {
                !w.cells[id as usize].biome.water()
                    && w.grid
                        .neighbors(id)
                        .any(|n| w.cells[n as usize].biome == Biome::Sea)
            })
            .unwrap();
        w.tribes.truncate(1);
        w.tribes[0].cell = cell;
        w.tribes[0].knowledge.seed(Skill::Watercraft, 0.8);
        w.tribes[0].knowledge.seed(Skill::Cordage, 0.8);
        w.tribes[0].knowledge.seed(Skill::Fishing, 0.8);
        for h in &mut w.tribes[0].people.members {
            for skill in [Skill::Watercraft, Skill::Cordage, Skill::Fishing] {
                h.skills.levels[skill as usize] = 0.8;
            }
        }
        w.cells[cell as usize].resources.wood = 0.;
        w.cells[cell as usize].resources.fiber = 0.;
        let before: f64 = w
            .cells
            .iter()
            .map(|c| f64::from(c.resources.aquatic.total()))
            .sum();
        daily(&mut w);
        let after: f64 = w
            .cells
            .iter()
            .map(|c| f64::from(c.resources.aquatic.total()))
            .sum();
        assert!(w.totals.fished > 0.);
        assert!((before - after - w.totals.fished).abs() < 0.1);
        assert_eq!(w.tribes[0].equipment.boat_places, 0.);
        w.tribes[0].equipment.wood = 100.;
        w.tribes[0].equipment.fiber = 100.;
        daily(&mut w);
        assert!(w.tribes[0].equipment.boat_places > 0.);
        assert!(w.tribes[0].equipment.wood < 100.);
    }

    #[test]
    fn crossings_require_knowledge_seats_range_and_safe_weather() {
        let mut w = world();
        let here = w.tribes[0].cell;
        let sea = w.grid.neighbors(here).next().unwrap();
        let land = w.grid.neighbors(sea).find(|&id| id != here).unwrap();
        w.cells[sea as usize].biome = Biome::Sea;
        w.cells[sea as usize].weather.snow = 0.;
        w.cells[sea as usize].weather.precipitation = 0.;
        w.cells[land as usize].biome = Biome::Grassland;
        let mut t = w.tribes[0].clone();
        t.travel = 100.;
        t.knowledge.seed(Skill::Watercraft, 0.8);
        t.knowledge.seed(Skill::Navigation, 0.8);
        t.equipment.boat_places = t.headcount() as f32;
        assert!(
            crossing_candidates(&w, &t)
                .iter()
                .any(|&(id, _)| id == land)
        );
        t.equipment.boat_places = 0.;
        assert!(crossing_candidates(&w, &t).is_empty());
        t.equipment.boat_places = t.headcount() as f32;
        t.knowledge.seed(Skill::Watercraft, 0.);
        assert!(crossing_candidates(&w, &t).is_empty());
        t.knowledge.seed(Skill::Watercraft, 0.8);
        t.travel = 0.;
        assert!(crossing_candidates(&w, &t).is_empty());
        t.travel = 100.;
        for cell in &mut w.cells {
            cell.weather.snow = 1.;
        }
        assert!(crossing_candidates(&w, &t).is_empty());
    }

    #[test]
    fn skilled_fishing_still_pays_for_search_travel_and_local_depletion() {
        let mut base = world();
        // Isolate fishing from seeded fauna and competing gathering jobs.
        base.herds.clear();
        base.tribes.truncate(1);
        let home = base.tribes[0].cell;
        let neighbor = base.grid.neighbors(home).next().unwrap();
        for cell in &mut base.cells {
            cell.resources.aquatic = AquaticLife::default();
            cell.resources.plants = 0.;
            cell.resources.fruit = 0.;
            cell.resources.game = 0.;
            cell.resources.foraging_pressure = 0.;
            cell.biome = Biome::Grassland;
            cell.weather.snow = 0.;
            cell.weather.precipitation = 0.;
        }
        let t = &mut base.tribes[0];
        t.health = 1.;
        t.residence_days = 30;
        t.knowledge.seed(Skill::Fishing, 1.);
        t.knowledge.seed(Skill::Cordage, 1.);
        for h in &mut t.people.members {
            h.skills.levels[Skill::Fishing as usize] = 1.;
            h.skills.levels[Skill::Cordage as usize] = 1.;
        }
        t.equipment.fiber = t.headcount() as f32;
        base.cells[home as usize].resources.aquatic.freshwater_fish = 5000.;
        let measure = |mut w: World| {
            let before = w.totals.fished;
            daily(&mut w);
            (w.totals.fished - before, w)
        };
        let (full, harvested) = measure(base.clone());
        assert!(full > 0.);
        assert!(harvested.cells[home as usize].resources.foraging_pressure > 0.);
        let mut sparse = base.clone();
        sparse.cells[home as usize]
            .resources
            .aquatic
            .freshwater_fish = FORAGING.aquatic_half_density
            * (base.config.cell_km / 20.).powi(2)
            * base.grid.area_factor(home)
            * 0.05;
        let (low, _) = measure(sparse);
        assert!(low < full * 0.5, "search slows before stock is exhausted");
        let mut depleted = base.clone();
        depleted.cells[home as usize].resources.foraging_pressure = 2.;
        assert!(measure(depleted).0 < full * 0.5);
        let mut distant = base.clone();
        distant.cells[home as usize]
            .resources
            .aquatic
            .freshwater_fish = 0.;
        distant.cells[neighbor as usize]
            .resources
            .aquatic
            .freshwater_fish = 5000.;
        assert!(measure(distant).0 < full);
        let mut moving = base;
        moving.tribes[0].residence_days = 0;
        assert!(measure(moving).0 < full * 0.75);
    }
}
