//! Residential fission without inventing people, possessions, or travel.
use humanity_core::*;
use std::collections::{BTreeMap, BTreeSet};

/// Count distinct days of friendly contact, not pair samples or headcount.
pub fn friendly_contact(a: &mut Tribe, b: &mut Tribe, day: u64) {
    let (a_culture, b_culture) = (a.culture, b.culture);
    for (t, other) in [(&mut *a, b_culture), (&mut *b, a_culture)] {
        if t.culture == other {
            t.last_cultural_contact = day;
        } else if t.last_foreign_contact != day {
            if t.contact_culture != Some(other) {
                t.contact_culture = Some(other);
                t.contact_days = 0;
            }
            t.contact_days = t.contact_days.saturating_add(1);
            t.last_foreign_contact = day;
        }
    }
}

pub fn daily(world: &mut World) {
    let year = world.config.year_days();
    let mut membership = vec![0_u32; world.cultures.len()];
    for t in &world.tribes {
        if t.headcount() > 0 {
            membership[t.culture as usize] += 1;
        }
    }
    let living_before = membership.clone();
    let mut mixtures: BTreeMap<_, _> = world
        .cultures
        .iter()
        .filter(|c| membership[c.id as usize] > 0 && c.parents.len() == 2)
        .map(|c| ((c.parents[0], c.parents[1]), c.id))
        .collect();
    for i in 0..world.tribes.len() {
        let t = &mut world.tribes[i];
        if t.headcount() == 0 {
            continue;
        }
        t.residence_days = t.residence_days.saturating_add(1);
        t.isolation_days = if t.last_cultural_contact == world.day {
            0
        } else {
            t.isolation_days.saturating_add(1)
        };
        if world.day.saturating_sub(t.last_foreign_contact)
            > u64::from(year) * u64::from(SOCIAL.contact_memory_years)
        {
            t.contact_days = 0;
            t.contact_culture = None;
        }
        let old = t.culture;
        let mixed = t.contact_culture.filter(|&id| {
            id != old
                && living_before[id as usize] > 0
                && t.contact_days >= SOCIAL.mixing_contact_days
        });
        let isolated =
            t.isolation_days >= year * SOCIAL.isolation_years && membership[old as usize] > 1;
        if mixed.is_none() && !isolated {
            continue;
        }
        let mut parents = vec![old];
        if let Some(other) = mixed {
            parents.push(other);
            parents.sort_unstable();
        }
        let existing = mixed.and_then(|_| mixtures.get(&(parents[0], parents[1])).copied());
        let new = if let Some(id) = existing {
            id
        } else {
            let inherited = if let Some(other) = mixed {
                if world.rng.chance(SOCIAL.mixing_other_variant_chance) {
                    other
                } else {
                    old
                }
            } else {
                old
            };
            let mut variant = world.cultures[inherited as usize].variant;
            if isolated && world.rng.chance(SOCIAL.variant_drift_chance) {
                let t = &world.tribes[i];
                variant = humanity_world::initial_culture_variant(
                    world,
                    t.cell,
                    t.id.wrapping_add(world.day as u32),
                );
            }
            let id = world.cultures.len() as u32;
            world.cultures.push(Culture {
                id,
                name: format!(
                    "{} {}",
                    if mixed.is_some() {
                        "Confluence"
                    } else {
                        "Branch"
                    },
                    id + 1
                ),
                variant,
                parents: parents.clone(),
                founded: world.day,
            });
            membership.push(0);
            if mixed.is_some() {
                mixtures.insert((parents[0], parents[1]), id);
            }
            id
        };
        let t = &mut world.tribes[i];
        membership[old as usize] -= 1;
        membership[new as usize] += 1;
        t.culture = new;
        t.camp_tradition = world.cultures[new as usize].variant.rules().camp;
        t.camp_pressure_days = 0;
        t.isolation_days = 0;
        t.last_cultural_contact = world.day;
        t.contact_culture = None;
        t.contact_days = 0;
        let event = Event {
            day: world.day,
            kind: if mixed.is_some() {
                EventKind::CultureMixing
            } else {
                EventKind::CultureDivergence
            },
            cell: t.cell,
            tribe: t.id,
            other: None,
            amount: new,
            probability: 1.,
        };
        if mixed.is_some() {
            world.totals.culture_mixtures += 1;
        } else {
            world.totals.culture_divergences += 1;
        }
        world.event(event);
    }
    sites(world);
}

fn sites(world: &mut World) {
    let live_bands: BTreeMap<_, _> = world
        .tribes
        .iter()
        .filter(|t| t.headcount() > 0)
        .map(|t| (t.id, t.cell))
        .collect();
    let live_cultures: BTreeSet<_> = world
        .tribes
        .iter()
        .filter(|t| t.headcount() > 0)
        .map(|t| t.culture)
        .collect();
    world
        .sites
        .retain(|s| live_bands.contains_key(&s.founder_band) || live_cultures.contains(&s.culture));
    let occupied: BTreeSet<_> = world
        .tribes
        .iter()
        .filter(|t| t.headcount() > 0)
        .map(|t| (t.culture, t.cell))
        .collect();
    let mut counts = vec![0; world.cultures.len()];
    let mut locations = BTreeMap::new();
    for (i, s) in world.sites.iter_mut().enumerate() {
        if occupied.contains(&(s.culture, s.cell))
            || live_bands.get(&s.founder_band) == Some(&s.cell)
        {
            s.last_visited = world.day;
        }
        counts[s.culture as usize] += 1;
        locations.insert((s.culture, s.cell, s.kind as u8, s.season), i);
    }
    for i in 0..world.tribes.len() {
        let t = &world.tribes[i];
        if t.headcount() == 0 {
            continue;
        }
        let season = world.local_season(t.cell);
        let residence = world.cultures[t.culture as usize].variant.rules().residence;
        let kind = match residence {
            Residence::Mobile => continue,
            Residence::Fishing => SiteKind::FishingSettlement,
            Residence::Seasonal => SiteKind::SeasonalCamp,
        };
        let site_season = if kind == SiteKind::SeasonalCamp {
            season
        } else {
            0
        };
        let key = (t.culture, t.cell, kind as u8, site_season);
        if let Some(&index) = locations.get(&key) {
            world.sites[index].last_visited = world.day;
            continue;
        }
        if counts[t.culture as usize] >= SOCIAL.max_sites_per_culture
            || t.residence_days < SOCIAL.site_founding_days
            || t.food < t.headcount() as f32 * SOCIAL.site_food_days
        {
            continue;
        }
        if kind == SiteKind::FishingSettlement {
            let aquatic: f32 = std::iter::once(t.cell)
                .chain(world.grid.neighbors(t.cell))
                .map(|c| &world.cells[c as usize])
                .filter(|c| c.biome != Biome::Ocean)
                .map(|c| c.resources.aquatic.total())
                .sum();
            if aquatic < t.headcount() as f32 * SOCIAL.fishing_site_aquatic_days
                || t.knowledge.level(Skill::Fishing) < SOCIAL.rich_fishing
            {
                continue;
            }
        }
        let site = CulturalSite {
            id: world.next_site_id,
            name: format!(
                "{} {}",
                if kind == SiteKind::FishingSettlement {
                    "Fish landing"
                } else {
                    "Season camp"
                },
                world.next_site_id + 1
            ),
            cell: t.cell,
            culture: t.culture,
            founder_band: t.id,
            kind,
            season: site_season,
            founded: world.day,
            last_visited: world.day,
        };
        let event = Event {
            day: world.day,
            kind: EventKind::SiteFounded,
            cell: t.cell,
            tribe: t.id,
            other: None,
            amount: t.culture,
            probability: 1.,
        };
        locations.insert(key, world.sites.len());
        counts[t.culture as usize] += 1;
        world.sites.push(site);
        world.next_site_id += 1;
        world.totals.sites_founded += 1;
        world.event(event);
    }
}

pub fn fission(world: &mut World) {
    // Appended daughters are visited too: oversized legacy bands resolve in a
    // single day, with work proportional to the resulting number of camps.
    let mut index = 0;
    while index < world.tribes.len() {
        let limit = world.camp_limit(&world.tribes[index]);
        let t = &mut world.tribes[index];
        let n = t.headcount();
        t.camp_pressure_days = if n > limit {
            t.camp_pressure_days
                .saturating_add(1)
                .min(t.camp_tradition.pressure_days())
        } else {
            t.camp_pressure_days
                .saturating_sub(SOCIAL.pressure_recovery)
        };
        let urgent = n > limit * SOCIAL.urgent_multiple;
        if n <= limit || (!urgent && t.camp_pressure_days < t.camp_tradition.pressure_days()) {
            index += 1;
            continue;
        }
        let Ok(id) = u32::try_from(world.next_band_id) else {
            break;
        };
        world.next_band_id += 1;
        // Dispersal releases small residential parties; balanced fission divides
        // the camp in two; budding retains a larger parent and a smaller offshoot.
        let rules = t.camp_tradition.rules();
        let mut departing = (n as f32 * rules.departing_fraction) as u32;
        if let Some(cap) = rules.departing_limit_fraction {
            departing = departing.min((limit as f32 * cap) as u32);
        }
        let departing = departing.clamp(1, n - 1);
        let mut daughter = split(t, id, departing);
        t.last_cultural_contact = world.day;
        t.isolation_days = 0;
        daughter.last_cultural_contact = world.day;
        daughter.isolation_days = 0;
        let event = Event {
            day: world.day,
            kind: EventKind::BandFission,
            cell: t.cell,
            tribe: t.id,
            other: Some(id),
            amount: departing,
            probability: 1.,
        };
        world.tribes.push(daughter);
        world.totals.band_fissions += 1;
        world.event(event);
        // Revisit an urgently oversized parent, otherwise let pressure rebuild.
        if !urgent {
            index += 1;
        }
    }
}

fn split(parent: &mut Tribe, id: u32, departing: u32) -> Tribe {
    let n = parent.headcount();
    let mut child = parent.clone();
    child.id = id;
    child.parent_band = Some(parent.id);
    parent.camp_pressure_days = 0;
    child.camp_pressure_days = 0;
    // Cumulative integer apportionment conserves every sex/age cohort exactly,
    // with at most one person's rounding error per cohort. Families are not
    // individually represented; both parties inherit the cohort's proficiency.
    let mut cumulative = 0_u64;
    let mut assigned = 0_u64;
    for (source, destination) in parent
        .people
        .female
        .iter_mut()
        .chain(&mut parent.people.male)
        .zip(child.people.female.iter_mut().chain(&mut child.people.male))
    {
        cumulative += u64::from(*source);
        let total = cumulative * u64::from(departing) / u64::from(n);
        *destination = (total - assigned) as u32;
        *source -= *destination;
        assigned = total;
    }
    let fraction = departing as f32 / n as f32;
    for (source, destination) in [
        (&mut parent.food, &mut child.food),
        (&mut parent.equipment.wood, &mut child.equipment.wood),
        (&mut parent.equipment.fiber, &mut child.equipment.fiber),
        (&mut parent.equipment.stone, &mut child.equipment.stone),
        (&mut parent.equipment.hides, &mut child.equipment.hides),
        (
            &mut parent.equipment.boat_places,
            &mut child.equipment.boat_places,
        ),
    ] {
        *destination = *source * fraction;
        *source -= *destination;
    }
    parent.specialists = Specialists::from_adults(parent.people.adults());
    child.specialists = Specialists::from_adults(child.people.adults());
    // Two camps may occupy one coarse cell; subsequent ordinary movement pays
    // actual travel costs. Fission grants no teleportation or bonus provisions.
    child
}

#[cfg(test)]
mod tests {
    use super::*;

    fn world() -> World {
        humanity_world::generate(PlanetConfig {
            resolution: 8,
            cell_km: 20.,
            temperature: 28.,
            season_days: 30,
            axial_tilt: 23.4,
            ocean_fraction: 0.6,
            rainfall: 1.,
            population: 1000,
            seed: 42,
        })
        .unwrap()
    }

    fn stock(t: &Tribe) -> [f32; 6] {
        [
            t.food,
            t.equipment.wood,
            t.equipment.fiber,
            t.equipment.stone,
            t.equipment.hides,
            t.equipment.boat_places,
        ]
    }

    #[test]
    fn fission_conserves_cohorts_possessions_and_knowledge_for_every_custom() {
        for tradition in CampTradition::ALL {
            let mut w = world();
            w.tribes.truncate(1);
            let t = &mut w.tribes[0];
            t.camp_tradition = tradition;
            t.people = Demography {
                female: [151, 311, 43],
                male: [157, 297, 44],
            };
            t.food = 10_000.;
            t.equipment = Equipment {
                wood: 512.,
                fiber: 255.,
                stone: 127.,
                hides: 99.,
                boat_places: 1024.,
            };
            let original = t.clone();
            let next = w.next_band_id;
            let births = w.totals.births;
            let deaths = w.totals.deaths;
            for _ in 0..=tradition.pressure_days() * 4 {
                w.day += 1;
                fission(&mut w);
            }
            assert!(w.tribes.len() > 2);
            assert_eq!(w.population(), u64::from(original.headcount()));
            assert_eq!((w.totals.births, w.totals.deaths), (births, deaths));
            assert_eq!(w.next_band_id, next + w.totals.band_fissions);
            for cohort in 0..3 {
                assert_eq!(
                    w.tribes
                        .iter()
                        .map(|t| t.people.female[cohort])
                        .sum::<u32>(),
                    original.people.female[cohort]
                );
                assert_eq!(
                    w.tribes.iter().map(|t| t.people.male[cohort]).sum::<u32>(),
                    original.people.male[cohort]
                );
            }
            for (i, before) in stock(&original).into_iter().enumerate() {
                let after: f32 = w.tribes.iter().map(|t| stock(t)[i]).sum();
                assert!((before - after).abs() < 0.01);
            }
            let ids: BTreeSet<_> = w.tribes.iter().map(|t| t.id).collect();
            assert_eq!(ids.len(), w.tribes.len());
            for t in &w.tribes {
                assert!(t.headcount() <= w.camp_limit(t));
                assert_eq!(t.cell, original.cell);
                assert_eq!(t.culture, original.culture);
                assert_eq!(t.camp_tradition, tradition);
                assert_eq!(t.knowledge.practitioners, original.knowledge.practitioners);
                assert_eq!(t.travel, original.travel);
                assert_eq!(
                    t.specialists.hunters
                        + t.specialists.gatherers
                        + t.specialists.healers
                        + t.specialists.guides,
                    t.people.adults()
                );
            }
        }
    }

    #[test]
    fn crowding_waits_for_custom_and_ecology_changes_threshold() {
        let mut w = world();
        w.tribes.truncate(1);
        let t = &mut w.tribes[0];
        t.camp_tradition = CampTradition::Balanced;
        t.people = Demography {
            female: [10, 25, 5],
            male: [10, 25, 5],
        };
        t.food = 800.;
        for _ in 1..CampTradition::Balanced.pressure_days() {
            fission(&mut w);
        }
        assert_eq!(w.tribes.len(), 1);
        fission(&mut w);
        assert_eq!(
            w.tribes.iter().map(Tribe::headcount).collect::<Vec<_>>(),
            vec![40, 40]
        );
        let t = &mut w.tribes[0];
        t.camp_tradition = CampTradition::RiverBudding;
        t.knowledge.seed(Skill::Fishing, 0.8);
        t.knowledge.seed(Skill::Preservation, 0.8);
        w.cells[t.cell as usize].resources.aquatic.freshwater_fish = 1e6;
        assert_eq!(w.camp_limit(&w.tribes[0]), SOCIAL.rich_camp_limit);
        w.tribes[0].food = 0.;
        assert_eq!(w.camp_limit(&w.tribes[0]), 75);
    }

    #[test]
    fn isolation_branches_but_friendly_contact_prevents_it() {
        let mut w = world();
        w.tribes.truncate(2);
        w.tribes[1].culture = w.tribes[0].culture;
        w.day = u64::from(w.config.year_days() * SOCIAL.isolation_years);
        w.tribes[0].isolation_days = w.day as u32 - 1;
        w.tribes[1].last_cultural_contact = w.day;
        let mut contacted = w.clone();
        let (a, b) = contacted.tribes.split_at_mut(1);
        friendly_contact(&mut a[0], &mut b[0], w.day);
        daily(&mut contacted);
        assert_eq!(contacted.tribes[0].culture, contacted.tribes[1].culture);
        let parent = w.tribes[0].culture;
        daily(&mut w);
        assert_ne!(w.tribes[0].culture, w.tribes[1].culture);
        assert_eq!(
            w.cultures[w.tribes[0].culture as usize].parents,
            vec![parent]
        );
        assert_eq!(w.totals.culture_divergences, 1);
    }

    #[test]
    fn repeated_contact_creates_one_shared_mixed_identity() {
        let mut w = world();
        w.tribes.truncate(2);
        w.tribes[0].culture = 0;
        w.tribes[1].culture = 1;
        for day in 1..=SOCIAL.mixing_contact_days {
            w.day = u64::from(day);
            let (a, b) = w.tribes.split_at_mut(1);
            for _ in 0..5 {
                friendly_contact(&mut a[0], &mut b[0], w.day);
            }
            assert_eq!(a[0].contact_days, day);
            daily(&mut w);
            if day < SOCIAL.mixing_contact_days {
                assert_ne!(w.tribes[0].culture, w.tribes[1].culture);
            }
        }
        assert_eq!(w.tribes[0].culture, w.tribes[1].culture);
        assert_eq!(w.cultures[w.tribes[0].culture as usize].parents, vec![0, 1]);
        assert_eq!(w.totals.culture_mixtures, 2);
    }

    #[test]
    fn remembered_locations_outlive_founder_but_not_its_entire_culture() {
        for variant in [
            CultureVariant::FishingResidents,
            CultureVariant::SeasonalRounds,
        ] {
            let mut w = world();
            w.tribes.truncate(2);
            w.cultures[0].variant = variant;
            for t in &mut w.tribes {
                t.culture = 0;
                t.camp_tradition = variant.rules().camp;
            }
            w.day = 20;
            let t = &mut w.tribes[0];
            t.residence_days = SOCIAL.site_founding_days;
            t.food = t.headcount() as f32 * 10.;
            t.knowledge.seed(Skill::Fishing, 0.8);
            w.cells[t.cell as usize].resources.aquatic.freshwater_fish = 1e6;
            daily(&mut w);
            assert_eq!(w.sites.len(), 1);
            let site = w.sites[0].clone();
            if variant == CultureVariant::SeasonalRounds {
                assert_eq!(site.season, w.local_season(site.cell));
            }
            w.tribes.remove(0);
            w.day += 1;
            daily(&mut w);
            assert_eq!(w.sites[0].id, site.id);
            w.tribes.clear();
            daily(&mut w);
            assert!(w.sites.is_empty());
        }
    }

    #[test]
    fn seasonal_return_obeys_local_season_and_food_security() {
        let mut base = world();
        base.tribes.truncate(1);
        base.herds.clear();
        let here = base.tribes[0].cell;
        let target = base
            .grid
            .neighbors(here)
            .find(|&c| !base.cells[c as usize].biome.water())
            .unwrap();
        for c in &mut base.cells {
            c.resources.plants = 1e6;
            c.resources.fruit = 0.;
            c.resources.game = 0.;
            c.resources.grazing = 0.;
            c.resources.aquatic = AquaticLife::default();
            c.river = 0.;
            c.weather.temperature = 18.;
            c.weather.snow = 0.;
        }
        let t = &mut base.tribes[0];
        t.food = t.headcount() as f32 * 10.;
        t.health = 1.;
        t.travel = base.config.cell_km * 2.;
        base.sites.push(CulturalSite {
            id: 0,
            name: "Return camp".into(),
            cell: target,
            culture: t.culture,
            founder_band: t.id,
            kind: SiteKind::SeasonalCamp,
            season: base.local_season(target),
            founded: 0,
            last_visited: 0,
        });
        let mut active = base.clone();
        crate::Simulation::default().move_tribes(&mut active);
        assert_eq!(active.tribes[0].cell, target);
        assert!(active.tribes[0].travel < base.tribes[0].travel);
        let mut inactive = base.clone();
        inactive.sites[0].season = (inactive.sites[0].season + 1) % 4;
        crate::Simulation::default().move_tribes(&mut inactive);
        assert_eq!(inactive.tribes[0].cell, here);
        base.tribes[0].food = 0.;
        crate::Simulation::default().move_tribes(&mut base);
        assert_eq!(base.tribes[0].cell, here);
    }
}
