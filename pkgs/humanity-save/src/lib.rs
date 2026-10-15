//! Versioned, human-inspectable snapshots. Validate before replacing live state.
use anyhow::{Context, Result, ensure};
use humanity_core::*;
use std::{
    collections::HashSet,
    fs::{self, File},
    io::{BufReader, BufWriter, Seek, SeekFrom, Write},
    path::Path,
};

pub fn save(world: &World, path: impl AsRef<Path>) -> Result<()> {
    validate(world)?;
    let path = path.as_ref();
    let parent = path
        .parent()
        .filter(|p| !p.as_os_str().is_empty())
        .unwrap_or(Path::new("."));
    fs::create_dir_all(parent).context("creating save directory")?;
    let mut temp =
        tempfile::NamedTempFile::new_in(parent).context("creating temporary snapshot")?;
    {
        let mut writer = BufWriter::new(temp.as_file_mut());
        serde_json::to_writer(&mut writer, world).context("serializing snapshot")?;
        writer.flush()?;
    }
    temp.as_file().sync_all()?;
    temp.persist(path)
        .context("atomically replacing snapshot")?;
    #[cfg(unix)]
    File::open(parent)?.sync_all()?;
    Ok(())
}

pub fn load(path: impl AsRef<Path>) -> Result<World> {
    let mut reader = BufReader::with_capacity(
        256 * 1024,
        File::open(path.as_ref()).context("opening snapshot")?,
    );
    #[derive(serde::Deserialize)]
    struct Header {
        version: u32,
    }
    let header: Header =
        serde_json::from_reader(&mut reader).context("reading snapshot version")?;
    ensure!(
        header.version == SAVE_VERSION,
        "unsupported save version {} (expected {SAVE_VERSION}); generate a new planet",
        header.version
    );
    reader.seek(SeekFrom::Start(0))?;
    let world: World = serde_json::from_reader(reader).context("decoding snapshot")?;
    validate(&world).context("invalid snapshot")?;
    Ok(world)
}

pub fn validate(w: &World) -> Result<()> {
    ensure!(
        w.version == SAVE_VERSION,
        "unsupported save version {} (expected {SAVE_VERSION})",
        w.version
    );
    w.config.validate().map_err(anyhow::Error::msg)?;
    ensure!(
        w.cells.len() == w.config.len(),
        "cell count disagrees with spherical resolution"
    );
    ensure!(
        w.grid.resolution() == w.config.resolution && w.grid.len() == w.cells.len(),
        "grid resolution mismatch"
    );
    ensure!(w.events.len() <= 512, "event history is oversized");
    let finite_range = |x: f32, min: f32, max: f32| x.is_finite() && (min..=max).contains(&x);
    let stock = |x| finite_range(x, 0., 1e15);
    for (i, c) in w.cells.iter().enumerate() {
        ensure!(
            finite_range(c.moisture, 0., 1.) && stock(c.river),
            "invalid climate at cell {i}"
        );
        ensure!(
            finite_range(c.weather.temperature, -200., 200.)
                && stock(c.weather.precipitation)
                && finite_range(c.weather.snow, 0., 1.)
                && finite_range(c.weather.soil_moisture, 0., 1.),
            "invalid weather at cell {i}"
        );
        ensure!(
            [
                c.resources.plants,
                c.resources.fruit,
                c.resources.game,
                c.resources.browse,
                c.resources.grazing,
                c.resources.aquatic.freshwater_fish,
                c.resources.aquatic.coastal_fish,
                c.resources.aquatic.shellfish,
                c.resources.wood,
                c.resources.fiber,
            ]
            .into_iter()
            .all(stock),
            "invalid resources at cell {i}"
        );
        ensure!(
            finite_range(c.resources.toolstone, 0., 1.)
                && c.resources.foraging_pressure.is_finite()
                && c.resources.foraging_pressure >= 0.,
            "invalid toolstone at cell {i}"
        );
        let v = &c.resources.vegetation;
        ensure!(
            v.vigor.iter().all(|&x| finite_range(x, 0., 1.))
                && finite_range(v.grass_cover, 0., 1.)
                && finite_range(v.woody_cover, 0., 1.)
                && v.grass_cover + v.woody_cover <= 1.00001
                && v.capacity
                    .values()
                    .into_iter()
                    .chain(v.wildlife_supply.values())
                    .all(stock),
            "invalid flora at cell {i}"
        );
        if let Some(downstream) = c.drainage {
            ensure!(
                w.grid.neighbors(i as u32).any(|n| n == downstream),
                "nonlocal drainage at cell {i}"
            );
        } else {
            ensure!(c.biome.water(), "land cell {i} has no drainage");
        }
    }
    // Linear-time graph validation, including inland lake links.
    let mut visited = vec![0_u8; w.cells.len()];
    for start in 0..w.cells.len() {
        let mut at = Some(start);
        while let Some(i) = at {
            if visited[i] != 0 {
                ensure!(visited[i] != 1, "drainage cycle at cell {i}");
                break;
            }
            visited[i] = 1;
            at = w.cells[i].drainage.map(|x| x as usize);
        }
        let mut at = Some(start);
        while let Some(i) = at {
            if visited[i] != 1 {
                break;
            }
            visited[i] = 2;
            at = w.cells[i].drainage.map(|x| x as usize);
        }
    }
    let valid_cell =
        |id: CellId| (id as usize) < w.cells.len() && !w.cells[id as usize].biome.water();
    let mut ids = HashSet::new();
    let mut locations = HashSet::new();
    let mut species_population = [0_u64; Species::ALL.len()];
    let mut small_game = vec![0_f32; w.cells.len()];
    ensure!(
        w.next_herd_id <= u64::from(u32::MAX) + 1,
        "invalid wildlife ID counter"
    );
    for h in &w.herds {
        ensure!(
            ids.insert(h.id)
                && valid_cell(h.cell)
                && u64::from(h.id) < w.next_herd_id
                && locations.insert((h.cell, h.species)),
            "invalid herd {}",
            h.id
        );
        ensure!(
            h.animals > 0
                && h.animals <= 1_000_000_000
                && finite_range(h.condition, 0., 1.)
                && finite_range(h.travel, 0., w.config.cell_km * 2.5)
                && finite_range(h.wariness, 0., 1.),
            "invalid herd state {}",
            h.id
        );
        species_population[h.species.index()] += u64::from(h.animals);
        if h.species.small() {
            small_game[h.cell as usize] += h.animals as f32 * h.species.meat();
        }
    }
    for species in Species::ALL {
        let i = species.index();
        ensure!(
            w.fauna_stats.initial[i]
                .checked_add(w.fauna_stats.births[i])
                .and_then(|n| n.checked_sub(w.fauna_stats.natural_deaths[i]))
                .and_then(|n| n.checked_sub(w.fauna_stats.harvested[i]))
                == Some(species_population[i]),
            "wildlife accounting mismatch for {:?}",
            species
        );
    }
    ensure!(
        w.fauna_stats
            .harvested
            .iter()
            .try_fold(0_u64, |n, &v| n.checked_add(v))
            == Some(w.totals.animals_hunted),
        "harvest accounting mismatch"
    );
    ensure!(
        w.cells
            .iter()
            .zip(small_game)
            .all(|(c, expected)| (c.resources.game - expected).abs() < 0.01 + expected * 0.00001),
        "small-game biomass must match living fauna"
    );
    ids.clear();
    ensure!(
        w.next_band_id <= u64::from(u32::MAX) + 1,
        "invalid next band ID"
    );
    for (i, c) in w.cultures.iter().enumerate() {
        ensure!(
            c.id as usize == i
                && !c.name.is_empty()
                && c.founded <= w.day
                && c.parents.len() <= 2
                && c.parents.iter().all(|&p| p < c.id)
                && (c.parents.len() != 2 || c.parents[0] < c.parents[1]),
            "invalid culture {}",
            c.id
        );
    }
    let mut living_cultures = HashSet::new();
    let mut human_ids = HashSet::new();
    for t in &w.tribes {
        ensure!(
            t.people.year_days == w.config.year_days() && t.people.day == w.day,
            "invalid human calendar"
        );
        let mut female = [0_u32; 3];
        let mut male = [0_u32; 3];
        let mut roles = [0_u32; 5];
        for h in &t.people.members {
            ensure!(
                human_ids.insert(h.id)
                    && h.id < w.next_human_id
                    && h.birth_day <= w.day as i64
                    && h.birth_day >= -i64::from(w.config.year_days()) * 200
                    && finite_range(h.health, 0., 1.)
                    && finite_range(h.work_fraction, 0., 1.)
                    && h.skills.levels.iter().all(|&v| finite_range(v, 0., 1.))
                    && h.skills
                        .experience
                        .iter()
                        .chain(&h.skills.taught)
                        .chain(&h.skills.stories)
                        .all(|&v| stock(v)),
                "invalid individual {}",
                h.id
            );
            let counts = if h.sex == Sex::Female {
                &mut female
            } else {
                &mut male
            };
            counts[h.stage(w.day, w.config.year_days())] += 1;
            match h.role {
                Role::Hunter => roles[0] += 1,
                Role::Gatherer => roles[1] += 1,
                Role::Fisher => roles[2] += 1,
                Role::Healer => roles[3] += 1,
                Role::Guide => roles[4] += 1,
                Role::Dependent => (),
            }
            let mut previous = 0;
            for event in &h.history {
                ensure!(
                    event.day >= previous && event.day <= w.day,
                    "invalid biography chronology"
                );
                previous = event.day;
                match event.kind {
                    LifeEventKind::TaughtBy { teacher, .. } => ensure!(
                        teacher < w.next_human_id && teacher != h.id,
                        "invalid teacher identity"
                    ),
                    LifeEventKind::ChildBorn(id) => {
                        ensure!(id < w.next_human_id && id != h.id, "invalid child identity")
                    }
                    LifeEventKind::JoinedBand(id) => ensure!(
                        u64::from(id) < w.next_band_id,
                        "invalid personal band history"
                    ),
                    LifeEventKind::SkillMilestone { level, .. } => {
                        ensure!(level <= 100, "invalid skill milestone")
                    }
                    _ => (),
                }
            }
        }
        ensure!(
            female == t.people.female && male == t.people.male,
            "individual/cohort count mismatch"
        );
        let summary = t.people.knowledge(&t.knowledge.prey);
        ensure!(
            summary
                .practitioners
                .iter()
                .flatten()
                .zip(t.knowledge.practitioners.iter().flatten())
                .all(|(a, b)| (a - b).abs() < 0.0001),
            "stale skill summary"
        );
        let health =
            t.people.members.iter().map(|h| h.health).sum::<f32>() / t.headcount().max(1) as f32;
        ensure!((health - t.health).abs() < 0.0001, "stale health summary");
        ensure!(
            (t.culture as usize) < w.cultures.len()
                && t.contact_culture
                    .is_none_or(|id| (id as usize) < w.cultures.len())
                && t.last_cultural_contact <= w.day
                && t.last_foreign_contact <= w.day,
            "invalid cultural membership in band {}",
            t.id
        );
        living_cultures.insert(t.culture);
        ensure!(
            u64::from(t.id) < w.next_band_id
                && t.parent_band.is_none_or(|id| id < t.id)
                && t.camp_pressure_days <= 1_000_000_000,
            "invalid residential state in band {}",
            t.id
        );
        ensure!(
            ids.insert(t.id) && valid_cell(t.cell),
            "invalid tribe {}",
            t.id
        );
        let population: u64 = t
            .people
            .female
            .iter()
            .chain(&t.people.male)
            .map(|&n| u64::from(n))
            .sum();
        ensure!(
            population > 0 && population <= 1_000_000_000,
            "invalid tribe population"
        );
        ensure!(
            stock(t.food)
                && finite_range(t.health, 0., 1.)
                && finite_range(t.hunting_skill, 0., 1.)
                && finite_range(t.travel, 0., w.config.cell_km * 2.),
            "invalid tribe state {}",
            t.id
        );
        let s = &t.specialists;
        ensure!(
            t.knowledge.prey.len() == Species::ALL.len()
                && t.knowledge
                    .practitioners
                    .iter()
                    .flatten()
                    .chain(&t.knowledge.prey)
                    .all(|&x| finite_range(x, 0., 1.)),
            "invalid knowledge in band {}",
            t.id
        );
        ensure!(
            [
                t.equipment.wood,
                t.equipment.fiber,
                t.equipment.stone,
                t.equipment.hides,
                t.equipment.boat_places
            ]
            .into_iter()
            .all(stock)
                && finite_range(t.fishing_effort, 0., 1.),
            "invalid equipment in band {}",
            t.id
        );
        ensure!(
            [s.hunters, s.gatherers, s.fishers, s.healers, s.guides] == roles,
            "specialist count mismatch"
        );
        ensure!(
            !t.languages.is_empty() && t.languages.len() <= 4,
            "invalid language mixture"
        );
        let mut families = HashSet::new();
        ensure!(
            t.languages
                .iter()
                .all(|l| finite_range(l.share, 0., 1.) && families.insert(l.family)),
            "invalid language share"
        );
        ensure!(
            (t.languages.iter().map(|l| l.share).sum::<f32>() - 1.).abs() < 0.001,
            "language shares must sum to one"
        );
    }
    let mut site_ids = HashSet::new();
    let mut site_keys = HashSet::new();
    for s in &w.sites {
        ensure!(
            (s.culture as usize) < w.cultures.len()
                && valid_cell(s.cell)
                && site_ids.insert(s.id)
                && s.id < w.next_site_id
                && !s.name.is_empty()
                && (1..=15).contains(&s.known_seasons)
                && finite_range(s.condition, 0., 1.)
                && s.last_maintained
                    .is_none_or(|day| day >= s.founded && day <= s.last_visited)
                && s.founded <= s.last_visited
                && s.last_visited <= w.day
                && u64::from(s.founder_band) < w.next_band_id
                && (living_cultures.contains(&s.culture) || ids.contains(&s.founder_band))
                && site_keys.insert((s.culture, s.cell)),
            "invalid cultural site {}",
            s.id
        );
        let o = &s.occupation;
        let since = o.since;
        ensure!(
            since >= s.founded
                && since <= w.day
                && o.occupied_days <= w.day - since + 1
                && o.episodes <= o.occupied_days
                && u64::from(o.longest_stay) <= o.occupied_days
                && o.continuous_days <= o.longest_stay
                && (o.present_people > 0) == (o.continuous_days > 0)
                && (o.present_people == 0 || s.last_visited == w.day)
                && !o.seasons.is_empty()
                && o.seasons.len() <= 8,
            "invalid site occupation {}",
            s.id
        );
        let period = w.day / u64::from(w.config.season_days);
        let mut previous = None;
        let mut days = 0_u64;
        for season in &o.seasons {
            ensure!(
                season.period <= period
                    && season.period.saturating_add(8) > period
                    && previous.is_none_or(|p| season.period == p + 1)
                    && season.season
                        == ((season.period + if w.grid.latitude(s.cell) < 0. { 2 } else { 0 }) % 4)
                            as u32
                    && season.occupied_days <= w.config.season_days
                    && season.person_days >= u64::from(season.occupied_days),
                "invalid site season history {}",
                s.id
            );
            previous = Some(season.period);
            days += u64::from(season.occupied_days);
        }
        ensure!(
            previous == Some(period) && days <= o.occupied_days,
            "stale site history {}",
            s.id
        );
    }
    for e in &w.events {
        if matches!(
            e.kind,
            EventKind::CultureMixing
                | EventKind::CultureDivergence
                | EventKind::SiteFounded
                | EventKind::SiteAbandoned
        ) {
            ensure!(
                (e.amount as usize) < w.cultures.len(),
                "invalid cultural event"
            );
        }
        if e.kind == EventKind::BandFission {
            ensure!(
                e.amount > 0 && e.other.is_some_and(|id| u64::from(id) < w.next_band_id),
                "invalid fission event"
            );
        }
        if matches!(e.kind, EventKind::Innovation | EventKind::KnowledgeExchange) {
            ensure!(
                (e.amount as usize) < Skill::ALL.len(),
                "invalid knowledge event topic"
            );
        }
        ensure!(
            e.day <= w.day
                && (e.cell as usize) < w.cells.len()
                && finite_range(e.probability, 0., 1.),
            "invalid event"
        );
    }
    ensure!(
        w.totals.gathered.is_finite()
            && w.totals.gathered >= 0.
            && w.totals.fished.is_finite()
            && w.totals.fished >= 0.,
        "invalid cumulative gathering"
    );
    let accounted = u64::from(w.config.population)
        .checked_add(w.totals.births)
        .and_then(|n| n.checked_sub(w.totals.deaths));
    ensure!(
        accounted == Some(w.population()),
        "population accounting mismatch"
    );
    ensure!(
        w.day < 1_000_000_000,
        "simulation day outside supported range"
    );
    validate_insights(w)?;
    Ok(())
}

fn validate_insights(w: &World) -> Result<()> {
    let d = &w.insights;
    let since = d
        .since_day
        .context("missing population diagnostics baseline")?;
    ensure!(
        since <= w.day && d.deaths.checked_total() == Some(w.totals.deaths),
        "death cause accounting mismatch"
    );
    if d.policy == AnalyticsPolicy::Off {
        ensure!(
            d.history.is_empty() && d.recent_deaths.is_empty() && w.events.is_empty(),
            "disabled analytics retains optional history"
        );
        return Ok(());
    }
    ensure!(
        !matches!(d.policy, AnalyticsPolicy::Years(0)),
        "analytics year limit must be positive"
    );
    ensure!(
        !d.history.is_empty()
            && d.policy
                .capacity(w.config.year_days())
                .is_none_or(|cap| d.history.len() <= cap)
            && d.recent_deaths.len() <= DEATH_RECORDS,
        "invalid diagnostic history size"
    );
    let mut previous: Option<&PopulationSample> = None;
    for s in &d.history {
        let deaths = s.deaths.checked_total().context("death count overflow")?;
        ensure!(
            s.day >= since
                && s.day <= w.day
                && s.births <= w.totals.births
                && u64::from(w.config.population)
                    .checked_add(s.births)
                    .and_then(|n| n.checked_sub(deaths))
                    == Some(s.population),
            "invalid historical population accounting"
        );
        ensure!(
            s.cohorts.iter().try_fold(0_u64, |n, x| n.checked_add(*x)) == Some(s.population)
                && s.low_health <= s.population
                && s.low_food <= s.population
                && s.mean_health.is_finite()
                && (0. ..=1.).contains(&s.mean_health),
            "invalid population sample"
        );
        ensure!(
            DeathCause::ALL
                .into_iter()
                .all(|c| s.deaths.get(c) <= d.deaths.get(c)),
            "historical deaths exceed totals"
        );
        if let Some(p) = previous {
            ensure!(
                s.day == p.day + 1
                    && s.births >= p.births
                    && DeathCause::ALL
                        .into_iter()
                        .all(|c| s.deaths.get(c) >= p.deaths.get(c)),
                "nonmonotonic population history"
            );
        }
        previous = Some(s);
    }
    let last = d.history.back().unwrap();
    ensure!(
        last.day == w.day
            && last.population == w.population()
            && last.births == w.totals.births
            && last.deaths == d.deaths,
        "stale population history"
    );
    let mut day = since;
    for r in &d.recent_deaths {
        ensure!(
            r.day >= day
                && r.day <= w.day
                && r.count > 0
                && (r.cell as usize) < w.cells.len()
                && r.health.is_finite()
                && (0. ..=1.).contains(&r.health)
                && r.food_days.is_finite()
                && r.food_days >= 0.
                && r.temperature.is_finite()
                && r.snow.is_finite()
                && (0. ..=1.).contains(&r.snow),
            "invalid death record"
        );
        day = r.day;
    }
    Ok(())
}

pub fn summary(w: &World) -> serde_json::Value {
    let mut culture_bands = vec![0_u64; w.cultures.len()];
    let mut culture_people = vec![0_u64; w.cultures.len()];
    for t in &w.tribes {
        culture_bands[t.culture as usize] += 1;
        culture_people[t.culture as usize] += u64::from(t.headcount());
    }
    let cultures: Vec<_> = w
        .cultures
        .iter()
        .filter(|c| culture_bands[c.id as usize] > 0)
        .map(|c| {
            serde_json::json!({ "id":c.id, "name":c.name, "variant":c.variant,
            "parents":c.parents, "founded":c.founded, "bands":culture_bands[c.id as usize],
            "population":culture_people[c.id as usize] })
        })
        .collect();
    let mut biomes = std::collections::BTreeMap::<String, u64>::new();
    let mut temperature = 0_f64;
    let mut snow = 0_u64;
    let mut rivers = 0_u64;
    for c in &w.cells {
        *biomes.entry(format!("{:?}", c.biome)).or_default() += 1;
        temperature += f64::from(c.weather.temperature);
        snow += u64::from(c.weather.snow > 0.2);
        rivers += u64::from(c.river > 0.);
    }
    let wildlife: std::collections::BTreeMap<_, _> = Species::ALL
        .into_iter()
        .map(|s| {
            (
                format!("{s:?}"),
                w.herds
                    .iter()
                    .filter(|h| h.species == s)
                    .map(|h| u64::from(h.animals))
                    .sum::<u64>(),
            )
        })
        .collect();
    let knowledge: std::collections::BTreeMap<_, _> = Skill::ALL
        .into_iter()
        .map(|s| {
            (
                format!("{s:?}"),
                w.tribes
                    .iter()
                    .map(|t| f64::from(t.knowledge.level(s)) * f64::from(t.people.adults()))
                    .sum::<f64>()
                    / w.tribes
                        .iter()
                        .map(|t| u64::from(t.people.adults()))
                        .sum::<u64>()
                        .max(1) as f64,
            )
        })
        .collect();
    let wildlife_diagnostics: std::collections::BTreeMap<_, _> = Species::ALL
        .into_iter()
        .map(|s| {
            let i = s.index();
            let population = wildlife[&format!("{s:?}")];
            let groups = w.herds.iter().filter(|h| h.species == s).count();
            let wariness = w
                .herds
                .iter()
                .filter(|h| h.species == s)
                .map(|h| f64::from(h.wariness) * f64::from(h.animals))
                .sum::<f64>()
                / population.max(1) as f64;
            (
                format!("{s:?}"),
                serde_json::json!({"animals": population, "groups": groups,
            "initial": w.fauna_stats.initial[i], "births": w.fauna_stats.births[i],
            "natural_deaths": w.fauna_stats.natural_deaths[i], "hunted": w.fauna_stats.harvested[i],
            "mean_wariness": wariness, "extinct": population == 0 && w.fauna_stats.initial[i] > 0}),
            )
        })
        .collect();
    let mut vegetation_cells = 0_u64;
    let mut vigor = [0_f64; 3];
    let mut supply = [0_f64; 3];
    for cell in &w.cells {
        let v = &cell.resources.vegetation;
        if v.grass_cover + v.woody_cover > 0. {
            vegetation_cells += 1;
            for (sum, value) in vigor.iter_mut().zip(v.vigor) {
                *sum += f64::from(value);
            }
        }
        for (sum, value) in supply.iter_mut().zip(v.wildlife_supply.values()) {
            *sum += f64::from(value);
        }
    }
    for value in &mut vigor {
        *value /= vegetation_cells.max(1) as f64;
    }
    let occupied: HashSet<_> = w.herds.iter().map(|h| h.cell).collect();
    let large_game: HashSet<_> = w
        .herds
        .iter()
        .filter(|h| !h.species.small())
        .map(|h| h.cell)
        .collect();
    serde_json::json!({
        "flora": { "vegetated_cells": vegetation_cells, "mean_vigor_grass_browse_fruit": vigor,
            "shared_daily_supply_grass_browse_fruit": supply,
            "wildlife_occupied_cells": occupied.len(), "large_game_cells": large_game.len() },
        "living_cultures": cultures,
        "cultural_sites": w.sites,
        "band_sizes": {
            "mean": w.population() as f64 / w.tribes.len().max(1) as f64,
            "largest": w.tribes.iter().map(Tribe::headcount).max().unwrap_or(0),
            "over_local_limit": w.tribes.iter().filter(|t| t.headcount() > w.camp_limit(t)).count(),
        },
        "camp_traditions": CampTradition::ALL.into_iter().map(|tradition| {
            (tradition.label(), w.tribes.iter().filter(|t| t.camp_tradition == tradition).count())
        }).collect::<std::collections::BTreeMap<_, _>>(),
        "wildlife":wildlife, "wildlife_diagnostics": wildlife_diagnostics,
        "wildlife_dispersals": w.fauna_stats.dispersals, "wildlife_mergers": w.fauna_stats.mergers,
        "adult_knowledge":knowledge,
        "aquatic_food":w.cells.iter().map(|c|f64::from(c.resources.aquatic.total())).sum::<f64>(),
        "boat_places":w.tribes.iter().map(|t|f64::from(t.equipment.boat_places)).sum::<f64>(),
        "day": w.day, "year": w.day / u64::from(w.config.year_days()) + 1,
        "cells": w.grid.len(), "pentagons": 12, "radius_km": w.config.radius_km(),
        "northern_season": w.season(), "population": w.population(), "tribes": w.tribes.len(),
        "animals": w.animals(), "herds": w.herds.len(), "biomes": biomes,
        "river_cells": rivers, "snow_cells": snow, "mean_temperature": temperature / w.cells.len() as f64,
        "mean_health": w.tribes.iter().map(|t| f64::from(t.health)).sum::<f64>() / w.tribes.len().max(1) as f64,
        "totals": w.totals, "recent_events": w.events.len(), "rng_state": w.rng.state,
        "death_causes": w.insights.deaths, "population_insights": w.insights.history.back(),
        "analytics_policy": w.insights.policy, "next_human_id": w.next_human_id,
    })
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
            population: 1000,
            seed: 42,
        })
        .unwrap()
    }
    #[test]
    fn atomic_save_resume_and_bad_state_rejection() {
        let dir = tempfile::tempdir().unwrap();
        let path = dir.path().join("world.json");
        let mut w = world();
        humanity_sim::Simulation::default().advance(&mut w, 70);
        // A real long-run aggregate that loses one ULP without float_roundtrip.
        w.totals.gathered = 28_652_685.687_526_926;
        save(&w, &path).unwrap();
        let mut restored = load(&path).unwrap();
        assert_eq!(
            w.totals.gathered.to_bits(),
            restored.totals.gathered.to_bits()
        );
        humanity_sim::Simulation::default().advance(&mut restored, 50);
        humanity_sim::Simulation::default().advance(&mut w, 50);
        assert_eq!(
            serde_json::to_value(&w).unwrap(),
            serde_json::to_value(&restored).unwrap()
        );
        save(&w, &path).unwrap();
        let saved = fs::read(&path).unwrap();
        w.tribes[0].cell = u32::MAX;
        assert!(save(&w, &path).is_err());
        assert_eq!(saved, fs::read(&path).unwrap());
        restored.version += 1;
        assert!(validate(&restored).is_err());
    }

    #[test]
    fn missing_diagnostics_and_corruption_are_rejected() {
        let mut w = world();
        humanity_sim::Simulation::default().advance(&mut w, 70);
        let mut old = serde_json::to_value(&w).unwrap();
        old.as_object_mut().unwrap().remove("insights");
        let dir = tempfile::tempdir().unwrap();
        let path = dir.path().join("old.json");
        fs::write(&path, serde_json::to_vec(&old).unwrap()).unwrap();
        assert!(load(&path).is_err());
        w.insights.deaths.nutrition += 1;
        assert!(validate(&w).is_err());
    }

    #[test]
    fn old_versions_are_rejected_before_decoding_world_state() {
        let dir = tempfile::tempdir().unwrap();
        let path = dir.path().join("old.json");
        for version in 1..SAVE_VERSION {
            fs::write(&path, format!("{{\"version\":{version}}}")).unwrap();
            assert!(
                load(&path)
                    .unwrap_err()
                    .to_string()
                    .contains("generate a new planet")
            );
        }
    }

    #[test]
    fn extinct_fauna_cannot_return_as_regenerated_meat_and_budgets_are_validated() {
        let mut w = world();
        for h in &mut w.herds {
            w.totals.animals_hunted += u64::from(h.harvest(u32::MAX, &mut w.fauna_stats));
        }
        w.herds.clear();
        humanity_world::wildlife::refresh_small_game(&mut w);
        humanity_sim::Simulation::default().advance(&mut w, 180);
        assert_eq!(w.animals(), 0);
        assert!(w.cells.iter().all(|c| c.resources.game == 0.));
        validate(&w).unwrap();
        w.fauna_stats.births[0] += 1;
        assert!(validate(&w).is_err());
    }

    #[test]
    fn analytics_modes_preserve_individual_state_and_resume_with_recording_off() {
        let mut full = world();
        let mut off = full.clone();
        off.set_analytics(AnalyticsPolicy::Off);
        humanity_sim::Simulation::default().advance(&mut full, 180);
        humanity_sim::Simulation::default().advance(&mut off, 180);
        assert_eq!(
            serde_json::to_value(&full.tribes).unwrap(),
            serde_json::to_value(&off.tribes).unwrap()
        );
        assert_eq!(
            serde_json::to_value(&full.cells).unwrap(),
            serde_json::to_value(&off.cells).unwrap()
        );
        assert_eq!(full.rng.state, off.rng.state);
        assert_eq!(full.insights.deaths, off.insights.deaths);
        assert!(off.insights.history.is_empty() && off.events.is_empty());
        assert!(
            off.tribes
                .iter()
                .all(|t| t.people.members.iter().all(|h| !h.history.is_empty()))
        );
        let dir = tempfile::tempdir().unwrap();
        let path = dir.path().join("individuals.json");
        save(&off, &path).unwrap();
        let mut resumed = load(&path).unwrap();
        humanity_sim::Simulation::default().advance(&mut off, 20);
        humanity_sim::Simulation::default().advance(&mut resumed, 20);
        assert_eq!(
            serde_json::to_value(&off).unwrap(),
            serde_json::to_value(&resumed).unwrap()
        );
        resumed.set_analytics(AnalyticsPolicy::Years(1));
        assert_eq!(resumed.insights.history.len(), 1);
        assert_eq!(resumed.insights.history[0].day, resumed.day);
        full.set_analytics(AnalyticsPolicy::Years(1));
        assert_eq!(
            full.insights.history.len(),
            full.config.year_days() as usize + 1
        );
        assert_eq!(
            full.insights.history.front().unwrap().day,
            full.day - u64::from(full.config.year_days())
        );
        let human = &mut resumed.tribes[0].people.members[0];
        human.skills.levels[0] = 1.5;
        assert!(validate(&resumed).is_err());
    }
}
