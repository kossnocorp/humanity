//! Live, two-column inspection of one residential band's individual members.
use crate::*;
use bevy::{
    input::mouse::{MouseScrollUnit, MouseWheel},
    window::PrimaryWindow,
};
use humanity_core::{Human, Skill, Tribe};

#[derive(Default)]
pub struct BandScreen {
    pub band: Option<u32>,
    human: Option<u64>,
    page_size: usize,
    revision: u64,
    reset_scroll: bool,
    notice: String,
}

#[derive(Component)]
pub struct Screen;
#[derive(Component)]
pub struct Details;
#[derive(Component, Clone, Copy)]
pub enum Action {
    Select(u64),
    Previous,
    Next,
    Close,
}

fn members(band: &Tribe) -> Vec<&Human> {
    let mut members: Vec<_> = band.people.members.iter().collect();
    members.sort_by_key(|h| h.id);
    members
}

impl BandScreen {
    fn select(&mut self, id: u64) {
        if self.human != Some(id) {
            self.human = Some(id);
            self.reset_scroll = true;
            self.notice.clear();
            self.revision = self.revision.wrapping_add(1);
        }
    }
    fn sync(&mut self, band: &Tribe) {
        let people = members(band);
        if !people.iter().any(|h| Some(h.id) == self.human) {
            let previous = self.human;
            self.human = people
                .iter()
                .find(|h| previous.is_none_or(|id| h.id >= id))
                .or(people.last())
                .map(|h| h.id);
            self.reset_scroll = true;
            if previous.is_some() {
                self.notice = "Previous selection died or left this band.".into();
            }
            self.revision = self.revision.wrapping_add(1);
        }
    }
    fn move_selection(&mut self, band: &Tribe, offset: isize) {
        let people = members(band);
        if people.is_empty() {
            return;
        }
        let current = people
            .iter()
            .position(|h| Some(h.id) == self.human)
            .unwrap_or(0);
        let next = current.saturating_add_signed(offset).min(people.len() - 1);
        self.select(people[next].id);
    }
}

pub fn toggle(world: &World, view: &mut ViewState) {
    if view.band_screen.band.is_some() {
        view.band_screen.band = None;
        return;
    }
    let band = view
        .following
        .and_then(|id| world.tribes.iter().find(|t| t.id == id))
        .or_else(|| {
            view.selected_human.and_then(|id| {
                world
                    .tribes
                    .iter()
                    .find(|t| t.people.members.iter().any(|h| h.id == id))
            })
        })
        .or_else(|| {
            view.selected.and_then(|cell| {
                world
                    .tribes
                    .iter()
                    .filter(|t| t.cell == cell)
                    .min_by_key(|t| t.id)
            })
        })
        .or_else(|| world.tribes.get(view.focus_index))
        .or_else(|| world.tribes.first());
    let Some(band) = band else {
        view.status = "No living band to inspect.".into();
        return;
    };
    view.band_screen = BandScreen {
        band: Some(band.id),
        human: view.selected_human,
        page_size: 8,
        reset_scroll: true,
        ..default()
    };
    view.band_screen.sync(band);
    view.insights_open = false;
    view.drag_start = None;
    view.last_cursor = None;
}

pub fn keyboard(keys: &ButtonInput<KeyCode>, world: &World, view: &mut ViewState) {
    let Some(band) = world
        .tribes
        .iter()
        .find(|t| Some(t.id) == view.band_screen.band)
    else {
        return;
    };
    let screen = &mut view.band_screen;
    screen.sync(band);
    let page = screen.page_size.max(1) as isize;
    for (key, delta) in [
        (KeyCode::ArrowUp, -1),
        (KeyCode::ArrowDown, 1),
        (KeyCode::PageUp, -page),
        (KeyCode::PageDown, page),
    ] {
        if keys.just_pressed(key) {
            screen.move_selection(band, delta);
        }
    }
    if keys.just_pressed(KeyCode::Home)
        && let Some(h) = members(band).first()
    {
        screen.select(h.id);
    }
    if keys.just_pressed(KeyCode::End)
        && let Some(h) = members(band).last()
    {
        screen.select(h.id);
    }
}

pub fn interact(
    game: Res<Game>,
    mut view: ResMut<ViewState>,
    buttons: Query<(&Interaction, &Action), Changed<Interaction>>,
) {
    if view.band_screen.band.is_none() {
        return;
    }
    for (interaction, action) in &buttons {
        if *interaction != Interaction::Pressed {
            continue;
        }
        match *action {
            Action::Close => view.band_screen.band = None,
            Action::Select(id) => {
                if game.world.tribes.iter().any(|t| {
                    Some(t.id) == view.band_screen.band
                        && t.people.members.iter().any(|h| h.id == id)
                }) {
                    view.band_screen.select(id);
                }
            }
            Action::Previous | Action::Next => {
                if let Some(band) = game
                    .world
                    .tribes
                    .iter()
                    .find(|t| Some(t.id) == view.band_screen.band)
                {
                    let step = view.band_screen.page_size.max(1) as isize;
                    view.band_screen.move_selection(
                        band,
                        if matches!(action, Action::Previous) {
                            -step
                        } else {
                            step
                        },
                    );
                }
            }
        }
    }
}

const TEXT: Color = Color::srgb(0.88, 0.9, 0.85);
const GOLD: Color = Color::srgb(0.95, 0.78, 0.49);
fn label(
    commands: &mut Commands,
    parent: Entity,
    text: impl Into<String>,
    size: f32,
    color: Color,
) {
    let e = commands
        .spawn((
            Text::new(text),
            TextFont {
                font_size: size,
                ..default()
            },
            TextColor(color),
            Node {
                flex_shrink: 0.,
                ..default()
            },
        ))
        .id();
    commands.entity(parent).add_child(e);
}
fn button(
    commands: &mut Commands,
    parent: Entity,
    text: impl Into<String>,
    action: Action,
    selected: bool,
) {
    let e = commands
        .spawn((
            Button,
            action,
            Node {
                padding: UiRect::axes(px(10), px(8)),
                flex_shrink: 0.,
                ..default()
            },
            BackgroundColor(if selected {
                Color::srgb(0.23, 0.34, 0.37)
            } else {
                Color::srgb(0.08, 0.13, 0.16)
            }),
        ))
        .id();
    commands.entity(parent).add_child(e);
    label(commands, e, text, 14., if selected { GOLD } else { TEXT });
}

fn biography(kind: &humanity_core::LifeEventKind) -> String {
    use humanity_core::LifeEventKind::*;
    match kind {
        Born => "Born".into(),
        FirstObserved => "First observed (earlier history unknown)".into(),
        ChildBorn(id) => format!("Gave birth to Human {id}"),
        JoinedBand(id) => format!("Joined Band {id}"),
        Birthday(age) => format!("Reached age {age}"),
        RoleChanged(role) => format!("Assigned {role:?}"),
        SkillMilestone { skill, level } => format!("{} reached level {level}", skill.label()),
        TaughtBy { teacher, skill } => format!("Learned {} from Human {teacher}", skill.label()),
    }
}

#[derive(Default)]
pub struct Cache {
    key: Option<(u64, u64, u64, u64, bool, u32, Vec2)>,
    updated: f64,
}

#[allow(clippy::too_many_arguments)]
pub fn update(
    mut commands: Commands,
    game: Res<Game>,
    mut view: ResMut<ViewState>,
    time: Res<Time>,
    window: Single<&Window, With<PrimaryWindow>>,
    roots: Query<Entity, With<Screen>>,
    details: Query<&ScrollPosition, With<Details>>,
    mut cache: Local<Cache>,
) {
    let Some(id) = view.band_screen.band else {
        for root in &roots {
            commands.entity(root).despawn();
        }
        cache.key = None;
        return;
    };
    let w = &game.world;
    let band = w.tribes.iter().find(|t| t.id == id);
    if let Some(band) = band {
        view.band_screen.sync(band);
    }
    let size = Vec2::new(window.width(), window.height());
    view.band_screen.page_size = ((size.y - 245.) / 65.).floor().clamp(1., 24.) as usize;
    let key = (
        w.day,
        view.snapshot_revision,
        u64::from(id),
        view.band_screen.revision,
        view.paused,
        view.speed.to_bits(),
        size,
    );
    if !roots.is_empty()
        && (cache.key == Some(key)
            || (cache.key.is_some_and(|k| {
                k.1 == key.1
                    && k.2 == key.2
                    && k.3 == key.3
                    && k.4 == key.4
                    && k.5 == key.5
                    && k.6 == key.6
            }) && time.elapsed_secs_f64() - cache.updated < 0.25))
    {
        return;
    }
    cache.key = Some(key);
    cache.updated = time.elapsed_secs_f64();
    let offset = if view.band_screen.reset_scroll {
        Vec2::ZERO
    } else {
        details.iter().next().map_or(Vec2::ZERO, |p| p.0)
    };
    view.band_screen.reset_scroll = false;
    for root in &roots {
        commands.entity(root).despawn();
    }
    let root = commands
        .spawn((
            Screen,
            GlobalZIndex(110),
            Interaction::None,
            Node {
                position_type: PositionType::Absolute,
                width: percent(100),
                height: percent(100),
                padding: UiRect::all(px(24)),
                flex_direction: FlexDirection::Column,
                row_gap: px(12),
                ..default()
            },
            BackgroundColor(Color::srgb(0.025, 0.045, 0.06)),
        ))
        .id();
    let header = commands
        .spawn(Node {
            justify_content: JustifyContent::SpaceBetween,
            align_items: AlignItems::Center,
            flex_shrink: 0.,
            ..default()
        })
        .id();
    commands.entity(root).add_child(header);
    label(
        &mut commands,
        header,
        format!("BAND {id} / INDIVIDUALS"),
        24.,
        GOLD,
    );
    button(
        &mut commands,
        header,
        "Close / I / Esc",
        Action::Close,
        false,
    );
    label(
        &mut commands,
        root,
        format!(
            "Day {}  |  {} {:.2} days/s  |  Space pause  -/+ speed  |  Up/Down select  PgUp/PgDn page",
            w.day,
            if view.paused { "PAUSED" } else { "LIVE" },
            view.speed
        ),
        14.,
        TEXT,
    );
    let Some(band) = band else {
        label(
            &mut commands,
            root,
            "This band no longer exists. Close this screen to choose another band.",
            18.,
            TEXT,
        );
        return;
    };
    label(
        &mut commands,
        root,
        format!(
            "{} members  |  Culture {}  |  Cell {}  |  Mean health {:.0}%  |  Food {:.1} days/person",
            band.headcount(),
            band.culture,
            band.cell,
            band.health * 100.,
            band.food / band.headcount().max(1) as f32
        ),
        14.,
        TEXT,
    );
    let body = commands
        .spawn(Node {
            flex_grow: 1.,
            min_height: px(0),
            column_gap: px(18),
            ..default()
        })
        .id();
    commands.entity(root).add_child(body);
    let left = commands
        .spawn((
            Node {
                width: percent(34),
                min_width: px(240),
                flex_shrink: 0.,
                flex_direction: FlexDirection::Column,
                row_gap: px(6),
                overflow: Overflow::clip(),
                ..default()
            },
            BackgroundColor(Color::srgb(0.045, 0.075, 0.095)),
        ))
        .id();
    commands.entity(body).add_child(left);
    let people = members(band);
    let current = people
        .iter()
        .position(|h| Some(h.id) == view.band_screen.human)
        .unwrap_or(0);
    let page_size = view.band_screen.page_size;
    let start = current / page_size * page_size;
    label(
        &mut commands,
        left,
        "MEMBERS / wheel or arrows to select",
        14.,
        GOLD,
    );
    for h in people.iter().skip(start).take(page_size) {
        button(
            &mut commands,
            left,
            format!(
                "Human {} / {:?} / age {}\n{:?} / health {:.0}%",
                h.id,
                h.sex,
                h.age(w.day, w.config.year_days()),
                h.role,
                h.health * 100.
            ),
            Action::Select(h.id),
            Some(h.id) == view.band_screen.human,
        );
    }
    let paging = commands
        .spawn(Node {
            column_gap: px(6),
            flex_wrap: FlexWrap::Wrap,
            flex_shrink: 0.,
            ..default()
        })
        .id();
    commands.entity(left).add_child(paging);
    button(&mut commands, paging, "< Previous", Action::Previous, false);
    label(
        &mut commands,
        paging,
        format!(
            "{} / {}",
            current / page_size + 1,
            people.len().div_ceil(page_size).max(1)
        ),
        14.,
        TEXT,
    );
    button(&mut commands, paging, "Next >", Action::Next, false);
    let right = commands
        .spawn((
            Details,
            Node {
                flex_grow: 1.,
                min_width: px(0),
                min_height: px(0),
                padding: UiRect::all(px(18)),
                flex_direction: FlexDirection::Column,
                row_gap: px(12),
                overflow: Overflow::scroll_y(),
                ..default()
            },
            ScrollPosition(offset),
            BackgroundColor(Color::srgb(0.055, 0.09, 0.115)),
        ))
        .id();
    commands.entity(body).add_child(right);
    if !view.band_screen.notice.is_empty() {
        label(&mut commands, right, &view.band_screen.notice, 14., GOLD);
    }
    let Some(h) = people.get(current) else {
        label(&mut commands, right, "No living members.", 18., TEXT);
        return;
    };
    label(
        &mut commands,
        right,
        format!("HUMAN {} / {:?}", h.id, h.sex),
        22.,
        GOLD,
    );
    label(
        &mut commands,
        right,
        format!(
            "Age {} years ({} days) / born day {}\nRole {:?} / health {:.1}% / working time {:.0}%\nBand {} / Culture {}\nScroll this panel for all skills and recent biography.",
            h.age(w.day, w.config.year_days()),
            h.age_days(w.day),
            h.birth_day,
            h.role,
            h.health * 100.,
            h.work_fraction * 100.,
            band.id,
            band.culture
        ),
        16.,
        TEXT,
    );
    label(
        &mut commands,
        right,
        "SKILLS / levels 0-100; experience in practiced days",
        16.,
        GOLD,
    );
    for skill in Skill::ALL {
        let i = skill as usize;
        label(
            &mut commands,
            right,
            format!(
                "{}: {:.1}\nPractice {:.1} days / instruction +{:.1} / stories +{:.1}",
                skill.label(),
                h.skills.level(skill) * 100.,
                h.skills.experience[i],
                h.skills.taught[i] * 100.,
                h.skills.stories[i] * 100.
            ),
            15.,
            TEXT,
        );
    }
    label(
        &mut commands,
        right,
        format!(
            "RECENT BIOGRAPHY / latest {} of {} events",
            h.history.len().min(30),
            h.history.len()
        ),
        16.,
        GOLD,
    );
    for event in h.history.iter().rev().take(30) {
        label(
            &mut commands,
            right,
            format!("Day {}: {}", event.day, biography(&event.kind)),
            14.,
            TEXT,
        );
    }
}

pub fn scroll(
    mut wheel: MessageReader<MouseWheel>,
    game: Res<Game>,
    mut view: ResMut<ViewState>,
    window: Single<&Window, With<PrimaryWindow>>,
    mut details: Query<(&mut ScrollPosition, &ComputedNode), With<Details>>,
) {
    for event in wheel.read() {
        if view.band_screen.band.is_none() {
            continue;
        }
        let Some(cursor) = window.cursor_position() else {
            continue;
        };
        if cursor.y < 140. {
            continue;
        }
        if cursor.x < 24. + ((window.width() - 48.) * 0.34).max(240.) {
            if let Some(band) = game
                .world
                .tribes
                .iter()
                .find(|t| Some(t.id) == view.band_screen.band)
            {
                view.band_screen.move_selection(
                    band,
                    if event.y > 0. {
                        -1
                    } else if event.y < 0. {
                        1
                    } else {
                        0
                    },
                );
            }
        } else {
            let delta = match event.unit {
                MouseScrollUnit::Line => event.y * 40.,
                MouseScrollUnit::Pixel => event.y,
            };
            for (mut position, node) in &mut details {
                let maximum =
                    (node.content_size().y - node.size().y).max(0.) * node.inverse_scale_factor();
                position.0.y = (position.0.y - delta).clamp(0., maximum);
            }
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use bevy::{ecs::system::RunSystemOnce, input::keyboard::KeyboardInput};

    fn world() -> World {
        humanity_world::generate(humanity_core::PlanetConfig {
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
        .unwrap()
    }

    #[test]
    fn selection_is_stable_across_reordering_and_handles_member_and_band_loss() {
        let mut w = world();
        let band = w.tribes[0].id;
        let id = members(&w.tribes[0])[1].id;
        let mut view = ViewState {
            following: Some(band),
            selected_human: Some(id),
            ..default()
        };
        toggle(&w, &mut view);
        assert_eq!(view.band_screen.band, Some(band));
        assert_eq!(view.band_screen.human, Some(id));
        w.tribes[0].people.members.reverse();
        view.band_screen.sync(&w.tribes[0]);
        assert_eq!(view.band_screen.human, Some(id));
        w.tribes[0].people.members.retain(|h| h.id != id);
        view.band_screen.sync(&w.tribes[0]);
        assert_ne!(view.band_screen.human, Some(id));
        assert!(!view.band_screen.notice.is_empty());
        w.tribes.remove(0);
        // Keep inspecting the original identity; don't silently jump to another band.
        assert_eq!(view.band_screen.band, Some(band));
        toggle(&w, &mut view);
        assert_eq!(view.band_screen.band, None);
        w.tribes.clear();
        view.following = None;
        view.selected_human = None;
        toggle(&w, &mut view);
        assert_eq!(view.band_screen.band, None);
    }

    #[test]
    fn i_controls_members_mouse_selection_and_time_without_rotating_globe() {
        let w = world();
        let band = w.tribes[0].id;
        let ids: Vec<_> = members(&w.tribes[0]).iter().map(|h| h.id).collect();
        let mut app = App::new();
        app.insert_resource(Game {
            world: w,
            simulation: Simulation::default(),
        })
        .insert_resource(ViewState {
            following: Some(band),
            speed: 4.,
            orbit: Quat::IDENTITY,
            ..default()
        })
        .init_resource::<Time>()
        .init_resource::<ButtonInput<KeyCode>>()
        .init_resource::<ButtonInput<MouseButton>>()
        .add_message::<KeyboardInput>()
        .add_message::<MouseWheel>();
        app.world_mut().spawn((Window::default(), PrimaryWindow));
        let press = |app: &mut App, key| {
            let mut keys = app.world_mut().resource_mut::<ButtonInput<KeyCode>>();
            keys.reset_all();
            keys.press(key);
            app.world_mut().run_system_once(input::controls).unwrap();
        };
        press(&mut app, KeyCode::KeyI);
        assert_eq!(
            app.world().resource::<ViewState>().band_screen.band,
            Some(band)
        );
        press(&mut app, KeyCode::ArrowDown);
        let view = app.world().resource::<ViewState>();
        assert_eq!(view.band_screen.human, Some(ids[1]));
        assert_eq!(view.orbit, Quat::IDENTITY);
        assert_eq!(view.following, Some(band));
        app.world_mut()
            .spawn((Action::Select(ids[2]), Interaction::Pressed));
        app.world_mut().run_system_once(interact).unwrap();
        assert_eq!(
            app.world().resource::<ViewState>().band_screen.human,
            Some(ids[2])
        );
        press(&mut app, KeyCode::Space);
        assert!(app.world().resource::<ViewState>().paused);
        press(&mut app, KeyCode::Equal);
        assert_eq!(app.world().resource::<ViewState>().speed, 8.);
        press(&mut app, KeyCode::Escape);
        let view = app.world().resource::<ViewState>();
        assert_eq!(view.band_screen.band, None);
        assert_eq!(view.following, Some(band));
    }
}
