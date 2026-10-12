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
        header.version == SAVE_VERSION || header.version == 2,
        "unsupported save version {} (expected {SAVE_VERSION}); cylindrical v1 worlds cannot be resumed on a spherical grid; generate a new planet",
        header.version
    );
    reader.seek(SeekFrom::Start(0))?;
    let mut world: World = serde_json::from_reader(reader).context("decoding snapshot")?;
    let upgrade = world.version == 2;
    world.version = SAVE_VERSION;
    validate(&world).context("invalid snapshot")?;
    if upgrade {
        humanity_world::upgrade_subsistence(&mut world);
    }
    if world.insights.since_day.is_none() {
        world.record_population();
    }
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
            finite_range(c.resources.toolstone, 0., 1.),
            "invalid toolstone at cell {i}"
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
    for h in &w.herds {
        ensure!(
            ids.insert(h.id) && valid_cell(h.cell),
            "invalid herd {}",
            h.id
        );
        ensure!(
            h.animals > 0
                && h.animals <= 1_000_000_000
                && finite_range(h.condition, 0., 1.)
                && finite_range(h.travel, 0., w.config.cell_km * 2.),
            "invalid herd state {}",
            h.id
        );
    }
    ids.clear();
    for t in &w.tribes {
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
            t.knowledge
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
            u64::from(s.hunters)
                + u64::from(s.gatherers)
                + u64::from(s.healers)
                + u64::from(s.guides)
                == u64::from(t.people.adults()),
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
    for e in &w.events {
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
    let Some(since) = d.since_day else {
        ensure!(
            d.history.is_empty()
                && d.recent_deaths.is_empty()
                && d.deaths == DeathCounts::default(),
            "incomplete population diagnostics"
        );
        return Ok(());
    };
    ensure!(
        since <= w.day && d.deaths.checked_total() == Some(w.totals.deaths),
        "death cause accounting mismatch"
    );
    ensure!(
        !d.history.is_empty()
            && d.history.len() <= HISTORY_DAYS
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
    serde_json::json!({
        "wildlife":wildlife, "adult_knowledge":knowledge,
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
    fn spherical_v2_upgrade_preserves_existing_state_and_runs_only_once() {
        let original = world();
        let mut value = serde_json::to_value(&original).unwrap();
        value["version"] = 2.into();
        for c in value["cells"].as_array_mut().unwrap() {
            for field in ["aquatic", "wood", "fiber", "toolstone"] {
                c["resources"].as_object_mut().unwrap().remove(field);
            }
        }
        for t in value["tribes"].as_array_mut().unwrap() {
            for field in ["knowledge", "equipment", "fishing_effort"] {
                t.as_object_mut().unwrap().remove(field);
            }
        }
        let dir = tempfile::tempdir().unwrap();
        let path = dir.path().join("old-sphere.json");
        fs::write(&path, serde_json::to_vec(&value).unwrap()).unwrap();
        let mut upgraded = load(&path).unwrap();
        assert_eq!(upgraded.rng.state, original.rng.state);
        assert_eq!(upgraded.population(), original.population());
        assert_eq!(upgraded.day, original.day);
        assert_eq!(
            upgraded.cells[10].resources.plants,
            original.cells[10].resources.plants
        );
        assert!(upgraded.tribes[0].knowledge.level(Skill::Knapping) > 0.);
        assert!(
            upgraded
                .cells
                .iter()
                .any(|c| c.resources.aquatic.total() > 0.)
        );
        upgraded.tribes[0].knowledge = Knowledge::default();
        save(&upgraded, &path).unwrap();
        let reloaded = load(&path).unwrap();
        assert_eq!(reloaded.tribes[0].knowledge.level(Skill::Knapping), 0.);
        assert_eq!(
            serde_json::to_string(&upgraded).unwrap(),
            serde_json::to_string(&reloaded).unwrap()
        );
        upgraded.tribes[0].knowledge.practitioners[0][1] = 1.1;
        assert!(validate(&upgraded).is_err());
    }
    #[test]
    fn atomic_save_resume_and_bad_state_rejection() {
        let dir = tempfile::tempdir().unwrap();
        let path = dir.path().join("world.json");
        let mut w = world();
        humanity_sim::Simulation::default().advance(&mut w, 70);
        save(&w, &path).unwrap();
        let mut restored = load(&path).unwrap();
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
    fn missing_diagnostics_get_an_honest_baseline_and_corruption_is_rejected() {
        let mut w = world();
        humanity_sim::Simulation::default().advance(&mut w, 70);
        let mut old = serde_json::to_value(&w).unwrap();
        old.as_object_mut().unwrap().remove("insights");
        let dir = tempfile::tempdir().unwrap();
        let path = dir.path().join("old.json");
        fs::write(&path, serde_json::to_vec(&old).unwrap()).unwrap();
        let mut restored = load(&path).unwrap();
        assert_eq!(restored.insights.deaths.unrecorded, w.totals.deaths);
        assert_eq!(restored.insights.since_day, Some(70));
        assert_eq!(restored.insights.history.len(), 1);
        humanity_sim::Simulation::default().advance(&mut restored, 30);
        validate(&restored).unwrap();
        restored.insights.deaths.nutrition += 1;
        assert!(validate(&restored).is_err());
    }

    #[test]
    fn cylindrical_versions_are_rejected_before_decoding_spherical_geometry() {
        let dir = tempfile::tempdir().unwrap();
        let path = dir.path().join("cylinder.json");
        fs::write(&path, r#"{"version":1,"config":{"width":128,"height":64}}"#).unwrap();
        let error = load(path).unwrap_err().to_string();
        assert!(error.contains("cylindrical"));
    }
}
