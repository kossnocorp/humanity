use anyhow::{Context, Result, bail};
use clap::Parser;
use humanity_core::PlanetConfig;
use std::{path::PathBuf, time::Instant};

/// Humanity: a procedural hunter-gatherer planet. One simulation turn is one day.
#[derive(Parser, Debug)]
#[command(version, about)]
struct Args {
    /// Icosahedron edge subdivisions. Cell count = 10 * resolution^2 + 2.
    #[arg(long, required_unless_present_any = ["load", "inspect"])]
    resolution: Option<u32>,
    /// Annual equatorial sea-level Celsius; proxy for solar heating.
    #[arg(long, allow_hyphen_values = true, required_unless_present_any = ["load", "inspect"])]
    temperature: Option<f32>,
    /// Days per season; four seasons per year.
    #[arg(long, required_unless_present_any = ["load", "inspect"])]
    season_days: Option<u32>,
    /// Initial humans, distributed into autonomous mobile bands.
    #[arg(long, required_unless_present_any = ["load", "inspect"])]
    population: Option<u32>,
    #[arg(long, default_value_t = 20.)]
    cell_km: f32,
    /// Set physical radius instead of mean cell spacing; spacing is derived from resolution.
    #[arg(long, conflicts_with = "cell_km")]
    radius_km: Option<f32>,
    #[arg(long, default_value_t = 23.4)]
    axial_tilt: f32,
    #[arg(long, default_value_t = 0.65)]
    ocean_fraction: f32,
    #[arg(long, default_value_t = 1.)]
    rainfall: f32,
    #[arg(long, default_value_t = 42)]
    seed: u64,
    /// Skip Bevy/window/GPU; final snapshot is saved automatically.
    #[arg(long)]
    headless: bool,
    /// Days to advance before opening the viewer, or headless days (default 365).
    #[arg(long)]
    days: Option<u64>,
    /// Emit a JSON summary every N headless days; zero means final only.
    #[arg(long, default_value_t = 30)]
    report_every: u64,
    /// Snapshot path for S/L and headless output.
    #[arg(long, default_value = "saves/world.json")]
    save: PathBuf,
    /// Resume all state and generation settings from this snapshot.
    #[arg(long, conflicts_with_all = ["inspect", "resolution", "temperature", "season_days", "population", "cell_km", "radius_km", "axial_tilt", "ocean_fraction", "rainfall", "seed"])]
    load: Option<PathBuf>,
    /// Validate a snapshot, print its summary and recent events, then exit.
    #[arg(long, conflicts_with = "load")]
    inspect: Option<PathBuf>,
    /// Viewer starts paused.
    #[arg(long)]
    paused: bool,
    /// Capture a rendered frame to PNG and exit (requires graphics/display).
    #[arg(long, conflicts_with = "headless")]
    screenshot: Option<PathBuf>,
    /// Open the population and mortality dashboard (also available with ?).
    #[arg(long, conflicts_with = "headless")]
    insights: bool,
    /// Begin the 3D viewer following this band ID (F cycles bands during play).
    #[arg(long, conflicts_with = "headless")]
    follow_band: Option<u32>,
    /// Open the viewer inspecting this living human and following their current band.
    #[arg(long, conflicts_with_all = ["headless", "follow_band"])]
    follow_human: Option<u64>,
    /// Optional analytics retention in simulated years; omitted means unlimited for new worlds.
    #[arg(long, conflicts_with_all = ["no_analytics", "analytics_unlimited"], value_parser = clap::value_parser!(u32).range(1..))]
    analytics_years: Option<u32>,
    /// Disable optional historical analytics, retaining current state and living biographies.
    #[arg(long, conflicts_with = "analytics_unlimited")]
    no_analytics: bool,
    /// Restore unlimited analytics recording, including when loading a capped/off save.
    #[arg(long)]
    analytics_unlimited: bool,
    /// Print a living individual's complete state and biography from --inspect.
    #[arg(long, requires = "inspect")]
    human: Option<u64>,
}

fn main() -> Result<()> {
    let args = Args::parse();
    if let Some(path) = &args.inspect {
        let world = humanity_save::load(path)?;
        if let Some(id) = args.human {
            let (band, human) = world
                .tribes
                .iter()
                .find_map(|t| {
                    t.people
                        .members
                        .iter()
                        .find(|h| h.id == id)
                        .map(|h| (t.id, h))
                })
                .with_context(|| format!("no living human with ID {id}"))?;
            println!(
                "{}",
                serde_json::to_string_pretty(
                    &serde_json::json!({"band": band, "age": human.age(world.day, world.config.year_days()), "human": human})
                )?
            );
            return Ok(());
        }
        println!(
            "{}",
            serde_json::to_string_pretty(&humanity_save::summary(&world))?
        );
        println!(
            "Recent events:\n{}",
            serde_json::to_string_pretty(&world.events.iter().rev().take(12).collect::<Vec<_>>())?
        );
        return Ok(());
    }
    #[cfg(not(feature = "graphics"))]
    if !args.headless {
        bail!("this build has no viewer; pass --headless or build with default features");
    }
    let start = Instant::now();
    let mut world = if let Some(path) = &args.load {
        humanity_save::load(path)?
    } else {
        let mut config = PlanetConfig {
            resolution: args.resolution.context("missing --resolution")?,
            temperature: args.temperature.context("missing --temperature")?,
            season_days: args.season_days.context("missing --season-days")?,
            population: args.population.context("missing --population")?,
            cell_km: args.cell_km,
            axial_tilt: args.axial_tilt,
            ocean_fraction: args.ocean_fraction,
            rainfall: args.rainfall,
            seed: args.seed,
        };
        config.validate().map_err(anyhow::Error::msg)?;
        if let Some(radius) = args.radius_km {
            if !radius.is_finite() || radius <= 0. {
                bail!("--radius-km must be positive and finite");
            }
            config.cell_km *= radius / config.radius_km();
        }
        humanity_world::generate(config).map_err(anyhow::Error::msg)?
    };
    if args.no_analytics {
        world.set_analytics(humanity_core::AnalyticsPolicy::Off);
    } else if let Some(years) = args.analytics_years {
        world.set_analytics(humanity_core::AnalyticsPolicy::Years(years));
    } else if args.analytics_unlimited {
        world.set_analytics(humanity_core::AnalyticsPolicy::Unlimited);
    }
    eprintln!(
        "Planet ready: {} cells, {} tribes, {} herds ({:.2}s)",
        world.cells.len(),
        world.tribes.len(),
        world.herds.len(),
        start.elapsed().as_secs_f64()
    );
    let days = args.days.unwrap_or(if args.headless { 365 } else { 0 });
    if days >= 1_000_000_000 - world.day {
        bail!("requested run exceeds the supported day range");
    }
    let start = Instant::now();
    let mut simulation = humanity_sim::Simulation::default();
    for advanced in 0..days {
        simulation.step(&mut world);
        if args.headless
            && args.report_every > 0
            && world.day % args.report_every == 0
            && advanced + 1 < days
        {
            println!("{}", humanity_save::summary(&world));
        }
    }
    if args.headless {
        humanity_save::save(&world, &args.save)?;
        println!("{}", humanity_save::summary(&world));
        eprintln!(
            "Advanced {days} days in {:.2}s; saved {}",
            start.elapsed().as_secs_f64(),
            args.save.display()
        );
        return Ok(());
    }
    #[cfg(feature = "graphics")]
    if let Some(id) = args.follow_band
        && !world.tribes.iter().any(|t| t.id == id)
    {
        bail!("no living band with ID {id}");
    }
    #[cfg(feature = "graphics")]
    let following = if let Some(id) = args.follow_human {
        Some(
            world
                .tribes
                .iter()
                .find(|t| t.people.members.iter().any(|h| h.id == id))
                .with_context(|| format!("no living human with ID {id}"))?
                .id,
        )
    } else {
        args.follow_band
    };
    #[cfg(feature = "graphics")]
    if !humanity_view::run(
        world,
        args.save,
        args.paused,
        args.screenshot,
        args.insights,
        following,
        args.follow_human,
    ) {
        bail!("viewer exited with an error");
    }
    Ok(())
}
