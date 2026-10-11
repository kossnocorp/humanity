use anyhow::{Context, Result, bail};
use clap::Parser;
use humanity_core::PlanetConfig;
use std::{path::PathBuf, time::Instant};

/// Humanity: a procedural hunter-gatherer planet. One simulation turn is one day.
#[derive(Parser, Debug)]
#[command(version, about)]
struct Args {
    /// Hex columns (longitude wraps).
    #[arg(long, required_unless_present_any = ["load", "inspect"])]
    width: Option<u32>,
    /// Hex rows, even (bounded poles).
    #[arg(long, required_unless_present_any = ["load", "inspect"])]
    height: Option<u32>,
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
    #[arg(long, conflicts_with_all = ["inspect", "width", "height", "temperature", "season_days", "population", "cell_km", "axial_tilt", "ocean_fraction", "rainfall", "seed"])]
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
}

fn main() -> Result<()> {
    let args = Args::parse();
    if let Some(path) = &args.inspect {
        let world = humanity_save::load(path)?;
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
        let config = PlanetConfig {
            width: args.width.context("missing --width")?,
            height: args.height.context("missing --height")?,
            temperature: args.temperature.context("missing --temperature")?,
            season_days: args.season_days.context("missing --season-days")?,
            population: args.population.context("missing --population")?,
            cell_km: args.cell_km,
            axial_tilt: args.axial_tilt,
            ocean_fraction: args.ocean_fraction,
            rainfall: args.rainfall,
            seed: args.seed,
        };
        humanity_world::generate(config).map_err(anyhow::Error::msg)?
    };
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
    if !humanity_view::run(
        world,
        args.save,
        args.paused,
        args.screenshot,
        args.insights,
    ) {
        bail!("viewer exited with an error");
    }
    Ok(())
}
