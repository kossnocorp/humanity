//! Bevy is an adapter over the simulation, never its source of truth.
mod assets;
mod input;
mod insights;
mod map;
mod navigation;
mod ui;

use bevy::prelude::*;
use humanity_core::{CellId, World};
use humanity_sim::Simulation;
use std::path::PathBuf;

/// Render units are independent of physical planet kilometers.
pub const PLANET_RADIUS: f32 = 100.;
pub const CAMERA_FOV: f32 = 0.8;

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
    Aquatic,
    Materials,
}
impl Layer {
    fn next(self) -> Self {
        match self {
            Self::Biomes => Self::Temperature,
            Self::Temperature => Self::Food,
            Self::Food => Self::Aquatic,
            Self::Aquatic => Self::Materials,
            Self::Materials => Self::Biomes,
        }
    }
    fn label(self) -> &'static str {
        match self {
            Self::Biomes => "BIOMES + SNOW",
            Self::Temperature => "TEMPERATURE",
            Self::Food => "EDIBLE RESOURCES",
            Self::Aquatic => "FISH + SHELLFISH (BIOMASS)",
            Self::Materials => "MATERIALS: STONE / WOOD / FIBER",
        }
    }
}

#[derive(Resource, Default)]
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
    insights_open: bool,
    following: Option<u32>,
    capture_exit: Option<AppExit>,
    snapshot_revision: u64,
    orbit: Quat,
    distance: f32,
    navigation: navigation::Navigator,
}

pub fn run(
    world: World,
    save_path: PathBuf,
    paused: bool,
    screenshot: Option<PathBuf>,
    insights_open: bool,
    following: Option<u32>,
) -> bool {
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
            insights_open,
            following,
            capture_exit: None,
            snapshot_revision: 0,
            orbit: Quat::IDENTITY,
            distance: PLANET_RADIUS * 3.2,
            navigation: navigation::Navigator::default(),
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
                navigation::interact,
                input::controls,
                tick,
                map::orbit_camera,
                map::redraw,
                ui::update,
                ui::scroll,
                navigation::update,
                insights::update,
                insights::scroll,
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

fn capture_frame(
    mut commands: Commands,
    mut view: ResMut<ViewState>,
    mut exit: MessageWriter<AppExit>,
) {
    use bevy::render::view::screenshot::{Screenshot, ScreenshotCaptured};
    // Screenshot observers run outside the normal update schedule. Emit exit
    // from Update so message maintenance cannot discard it before the runner sees it.
    if let Some(result) = &view.capture_exit {
        exit.write(result.clone());
        return;
    }
    view.frames = view.frames.saturating_add(1);
    if view.frames == 30
        && let Some(path) = view.screenshot.take()
    {
        commands.spawn(Screenshot::primary_window()).observe(
            move |capture: On<ScreenshotCaptured>, mut view: ResMut<ViewState>| {
                if let Some(parent) = path.parent().filter(|p| !p.as_os_str().is_empty())
                    && let Err(error) = std::fs::create_dir_all(parent)
                {
                    eprintln!("Screenshot directory: {error}");
                    view.capture_exit = Some(AppExit::error());
                    return;
                }
                match capture.image.clone().try_into_dynamic() {
                    Ok(image) => match image.save(&path) {
                        Ok(()) => {
                            eprintln!("Saved screenshot {}", path.display());
                            view.capture_exit = Some(AppExit::Success);
                        }
                        Err(error) => {
                            eprintln!("Screenshot: {error}");
                            view.capture_exit = Some(AppExit::error());
                        }
                    },
                    Err(error) => {
                        eprintln!("Screenshot: {error}");
                        view.capture_exit = Some(AppExit::error());
                    }
                }
            },
        );
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use bevy::ecs::system::RunSystemOnce;
    #[test]
    fn charts_do_not_pause_time_and_keyboard_controls_remain_live() {
        use bevy::input::{keyboard::KeyboardInput, mouse::MouseWheel};
        use bevy::window::PrimaryWindow;
        let world = humanity_world::generate(humanity_core::PlanetConfig {
            resolution: 6,
            cell_km: 20.,
            temperature: 28.,
            season_days: 30,
            axial_tilt: 23.4,
            ocean_fraction: 0.6,
            rainfall: 1.,
            population: 500,
            seed: 42,
        })
        .unwrap();
        let mut app = App::new();
        app.insert_resource(Game {
            world,
            simulation: Simulation::default(),
        })
        .insert_resource(ViewState {
            insights_open: true,
            speed: 4.,
            ..default()
        })
        .init_resource::<Time>()
        .init_resource::<ButtonInput<KeyCode>>()
        .init_resource::<ButtonInput<MouseButton>>()
        .add_message::<KeyboardInput>()
        .add_message::<MouseWheel>();
        app.world_mut().spawn((Window::default(), PrimaryWindow));
        app.world_mut().spawn((
            Camera3d::default(),
            Transform::default(),
            Projection::Perspective(PerspectiveProjection::default()),
        ));
        app.world_mut()
            .resource_mut::<Time>()
            .advance_by(std::time::Duration::from_secs_f32(0.25));
        app.world_mut().run_system_once(tick).unwrap();
        assert_eq!(app.world().resource::<Game>().world.day, 1);
        app.world_mut()
            .resource_mut::<ButtonInput<KeyCode>>()
            .press(KeyCode::Space);
        app.world_mut().run_system_once(input::controls).unwrap();
        app.world_mut().run_system_once(tick).unwrap();
        assert_eq!(app.world().resource::<Game>().world.day, 1);
        assert!(app.world().resource::<ViewState>().paused);
        app.world_mut()
            .resource_mut::<ButtonInput<KeyCode>>()
            .reset_all();
        app.world_mut()
            .resource_mut::<ButtonInput<KeyCode>>()
            .press(KeyCode::Equal);
        app.world_mut().run_system_once(input::controls).unwrap();
        assert_eq!(app.world().resource::<ViewState>().speed, 8.);
        app.world_mut()
            .resource_mut::<ButtonInput<KeyCode>>()
            .reset_all();
        app.world_mut()
            .resource_mut::<ButtonInput<KeyCode>>()
            .press(KeyCode::Minus);
        app.world_mut().run_system_once(input::controls).unwrap();
        assert_eq!(app.world().resource::<ViewState>().speed, 4.);
        app.world_mut()
            .resource_mut::<ButtonInput<KeyCode>>()
            .reset_all();
        app.world_mut()
            .resource_mut::<ButtonInput<KeyCode>>()
            .press(KeyCode::Space);
        app.world_mut().run_system_once(input::controls).unwrap();
        app.world_mut().run_system_once(tick).unwrap();
        assert_eq!(app.world().resource::<Game>().world.day, 2);
        assert!(app.world().resource::<ViewState>().insights_open);
        app.world_mut()
            .resource_mut::<ButtonInput<KeyCode>>()
            .reset_all();
        app.world_mut()
            .resource_mut::<ButtonInput<KeyCode>>()
            .press(KeyCode::ShiftLeft);
        app.world_mut()
            .resource_mut::<ButtonInput<KeyCode>>()
            .press(KeyCode::Slash);
        app.world_mut().run_system_once(input::controls).unwrap();
        assert!(!app.world().resource::<ViewState>().insights_open);
        assert!(!app.world().resource::<ViewState>().paused);
        // Menu navigation must not rotate the globe or release band following.
        let band = app.world().resource::<Game>().world.tribes[0].id;
        {
            let mut view = app.world_mut().resource_mut::<ViewState>();
            view.following = Some(band);
            view.orbit = Quat::IDENTITY;
            view.distance = PLANET_RADIUS * 3.;
        }
        {
            let mut keys = app.world_mut().resource_mut::<ButtonInput<KeyCode>>();
            keys.reset_all();
            keys.press(KeyCode::AltLeft);
            keys.press(KeyCode::ArrowRight);
        }
        app.world_mut().run_system_once(input::controls).unwrap();
        assert_eq!(app.world().resource::<ViewState>().following, Some(band));
        assert_eq!(app.world().resource::<ViewState>().orbit, Quat::IDENTITY);
    }
}
