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
    if !learner.knowledge.prerequisites(skill) {
        return None;
    }
    let gap = teacher.knowledge.level(skill) - learner.knowledge.level(skill);
    if gap < 0.02 {
        return None;
    }
    let learned = gap * (0.08 + 0.12 * language);
    learner.knowledge.practitioners[skill as usize][1] += learned;
    for (a, b) in learner
        .knowledge
        .prey
        .iter_mut()
        .zip(teacher.knowledge.prey)
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

/// Within-band apprenticeship and oral teaching. Elders can preserve a practice
/// even when few working adults know it. No teachers means no spontaneous spread.
pub fn teach(t: &mut Tribe, year: f32) {
    let adults = t.people.adults() as f32;
    let elders = (t.people.female[2] + t.people.male[2]) as f32;
    let oral = t.knowledge.level(Skill::OralTradition);
    for k in &mut t.knowledge.practitioners {
        let teachers = (k[1] * adults + k[2] * elders * 1.5) / (adults + elders * 1.5).max(1.);
        for (cohort, rate) in [(0, 3.), (1, 1.8), (2, 0.6)] {
            let gain = teachers * (1. - k[cohort]) * rate * (0.4 + oral) * t.health / year;
            k[cohort] = (k[cohort] + gain - k[cohort] * 0.04 / year).clamp(0., 1.);
        }
    }
}

pub fn daily(world: &mut World) {
    let mut events = Vec::new();
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
        teach(t, world.config.year_days() as f32);
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
                t.knowledge.practitioners[skill as usize][1] =
                    (t.knowledge.level(skill) + 1. / t.people.adults() as f32).min(1.);
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
        let e = &mut t.equipment;
        let labor = t.people.adults() as f32 * t.health;
        // Ten percent of labor supports tools, containers and transport.
        let wood = c
            .resources
            .wood
            .min(labor * 0.04)
            .min((n * 2. - e.wood).max(0.));
        let fiber = c
            .resources
            .fiber
            .min(labor * 0.04)
            .min((n - e.fiber).max(0.));
        c.resources.wood -= wood;
        e.wood += wood;
        c.resources.fiber -= fiber;
        e.fiber += fiber;
        e.stone = (e.stone + c.resources.toolstone * labor * 0.02).min(n);
        e.stone *= 0.998;
        e.hides *= 0.999;
        e.boat_places *= 0.999;
        let craft = t.knowledge.level(Skill::Watercraft);
        if water && craft > 0.15 && t.knowledge.level(Skill::Cordage) > 0.1 {
            let places = (labor * craft * 0.08)
                .min(e.wood / 2.)
                .min(e.fiber)
                .min((n * 1.15 - e.boat_places).max(0.));
            e.wood -= places * 2.;
            e.fiber -= places;
            e.boat_places += places;
        }
        t.fishing_effort = if water {
            (0.12 + t.knowledge.level(Skill::Fishing) * 0.5).min(0.6)
        } else {
            0.
        };
        let fishing = t.knowledge.level(Skill::Fishing);
        let gear = t.knowledge.level(Skill::Cordage) * (e.fiber / (n * 0.1).max(1.)).min(1.);
        let mut budget = labor * 0.9 * t.fishing_effort * (1. + fishing * 5. + gear * 2.);
        let mut catch = 0.;
        for id in sites {
            let site = &mut world.cells[id as usize];
            if site.biome == Biome::Ocean && (e.boat_places < n * 0.25 || craft < 0.2) {
                continue;
            }
            let access = (1. - site.weather.snow * 0.9) / (1. + site.weather.precipitation * 0.08);
            for (stock, efficiency) in [
                (&mut site.resources.aquatic.shellfish, 0.8),
                (&mut site.resources.aquatic.freshwater_fish, fishing),
                (&mut site.resources.aquatic.coastal_fish, fishing),
            ] {
                let take = stock.min(budget * efficiency * access * (0.6 + world.rng.unit() * 0.4));
                *stock -= take;
                catch += take;
                budget -= take;
            }
        }
        e.fiber = (e.fiber - labor * t.fishing_effort * 0.004).max(0.);
        t.food += catch;
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
        t.knowledge = Knowledge::default();
        t.knowledge.practitioners[Skill::Fishing as usize][2] = 1.;
        teach(&mut t, 120.);
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
}
