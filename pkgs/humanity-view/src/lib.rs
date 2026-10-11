//! Bevy is an adapter over the simulation, never its source of truth.
mod assets;
mod input;
mod map;
mod ui;

use bevy::prelude::*;
use humanity_core::{CellId, World};
use humanity_sim::Simulation;
use std::path::PathBuf;

pub const RADIUS: f32 = 16.;
pub const HEX_WIDTH: f32 = RADIUS * 1.732_050_8;

#[derive(Resource)]
struct Game {
    world: World,
    simulation: Simulation,
}

#[derive(Clone, Copy, Default)]
enum Layer {
    #[default]
    Biomes,
    Temperature,
    Food,
}
impl Layer {
    fn next(self) -> Self {
        match self {
            Self::Biomes => Self::Temperature,
            Self::Temperature => Self::Food,
            Self::Food => Self::Biomes,
        }
    }
    fn label(self) -> &'static str {
        match self {
            Self::Biomes => "BIOMES + SNOW",
            Self::Temperature => "TEMPERATURE",
            Self::Food => "EDIBLE RESOURCES",
        }
    }
}

#[derive(Resource)]
struct ViewState {
    paused: bool,
    speed: f32,
    accumulator: f32,
    save_path: PathBuf,
    status: String,
    selected: Option<CellId>,
    layer: Layer,
    dirty: bool,
    last_draw: f64,
    last_day: u64,
    last_size: Vec2,
    drag_start: Option<Vec2>,
    last_cursor: Option<Vec2>,
    dragged: bool,
    focus_index: usize,
    lod: usize,
    screenshot: Option<PathBuf>,
    frames: u32,
}

pub fn run(world: World, save_path: PathBuf, paused: bool, screenshot: Option<PathBuf>) -> bool {
    App::new()
        .insert_resource(ClearColor(Color::srgb(0.035, 0.06, 0.085)))
        .insert_resource(Game {
            world,
            simulation: Simulation::default(),
        })
        .insert_resource(ViewState {
            paused,
            speed: 4.,
            accumulator: 0.,
            save_path,
            status: "Click a hex to inspect its ecology and people.".into(),
            selected: None,
            layer: Layer::default(),
            dirty: true,
            last_draw: -1.,
            last_day: u64::MAX,
            last_size: Vec2::ZERO,
            drag_start: None,
            last_cursor: None,
            dragged: false,
            focus_index: 0,
            lod: 1,
            screenshot,
            frames: 0,
        })
        .add_plugins(DefaultPlugins.set(WindowPlugin {
            primary_window: Some(Window {
                title: "Humanity · Before the first settlements".into(),
                resolution: (1440, 900).into(),
                ..default()
            }),
            ..default()
        }))
        .add_systems(Startup, (assets::setup, map::setup, ui::setup).chain())
        .add_systems(
            Update,
            (
                input::controls,
                tick,
                map::redraw,
                ui::update,
                capture_frame,
            )
                .chain(),
        )
        .run()
        .is_success()
}

fn tick(time: Res<Time>, mut game: ResMut<Game>, mut view: ResMut<ViewState>) {
    if view.paused {
        return;
    }
    view.accumulator = (view.accumulator + time.delta_secs().min(0.25) * view.speed).min(32.);
    let start = std::time::Instant::now();
    let Game { world, simulation } = &mut *game;
    let mut steps = 0;
    while view.accumulator >= 1. && steps < 8 {
        if world.day >= 999_999_998 {
            view.paused = true;
            break;
        }
        simulation.step(world);
        view.accumulator -= 1.;
        steps += 1;
        if start.elapsed().as_millis() >= 8 {
            break;
        }
    }
}

fn capture_frame(mut commands: Commands, mut view: ResMut<ViewState>) {
    use bevy::render::view::screenshot::{Screenshot, ScreenshotCaptured};
    view.frames = view.frames.saturating_add(1);
    if view.frames == 30
        && let Some(path) = view.screenshot.take()
    {
        commands.spawn(Screenshot::primary_window()).observe(
            move |capture: On<ScreenshotCaptured>, mut exit: MessageWriter<AppExit>| {
                if let Some(parent) = path.parent().filter(|p| !p.as_os_str().is_empty())
                    && let Err(error) = std::fs::create_dir_all(parent)
                {
                    eprintln!("Screenshot directory: {error}");
                    exit.write(AppExit::error());
                    return;
                }
                match capture.image.clone().try_into_dynamic() {
                    Ok(image) => match image.save(&path) {
                        Ok(()) => {
                            eprintln!("Saved screenshot {}", path.display());
                            exit.write(AppExit::Success);
                        }
                        Err(error) => {
                            eprintln!("Screenshot: {error}");
                            exit.write(AppExit::error());
                        }
                    },
                    Err(error) => {
                        eprintln!("Screenshot: {error}");
                        exit.write(AppExit::error());
                    }
                }
            },
        );
    }
}
