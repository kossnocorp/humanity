//! Versioned, human-inspectable snapshots. Validate before replacing live state.
use anyhow::{Context, Result, ensure};
use humanity_core::*;
use std::{
    collections::HashSet,
    fs::{self, File},
    io::{BufReader, BufWriter, Write},
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
    let reader = BufReader::with_capacity(
        256 * 1024,
        File::open(path.as_ref()).context("opening snapshot")?,
    );
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
        "cell count disagrees with dimensions"
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
                c.resources.grazing
            ]
            .into_iter()
            .all(stock),
            "invalid resources at cell {i}"
        );
        if let Some(downstream) = c.drainage {
            ensure!(
                w.config.neighbors(i as u32).any(|n| n == downstream),
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
        ensure!(
            e.day <= w.day
                && (e.cell as usize) < w.cells.len()
                && finite_range(e.probability, 0., 1.),
            "invalid event"
        );
    }
    ensure!(
        w.totals.gathered.is_finite() && w.totals.gathered >= 0.,
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
    serde_json::json!({
        "day": w.day, "year": w.day / u64::from(w.config.year_days()) + 1,
        "northern_season": w.season(), "population": w.population(), "tribes": w.tribes.len(),
        "animals": w.animals(), "herds": w.herds.len(), "biomes": biomes,
        "river_cells": rivers, "snow_cells": snow, "mean_temperature": temperature / w.cells.len() as f64,
        "mean_health": w.tribes.iter().map(|t| f64::from(t.health)).sum::<f64>() / w.tribes.len().max(1) as f64,
        "totals": w.totals, "recent_events": w.events.len(), "rng_state": w.rng.state,
    })
}

#[cfg(test)]
mod tests {
    use super::*;
    fn world() -> World {
        humanity_world::generate(PlanetConfig {
            width: 48,
            height: 32,
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
}
