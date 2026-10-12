//! Bounded, paginated entity browser. Entries use stable simulation IDs.
use crate::*;
use bevy::{input::mouse::MouseWheel, window::PrimaryWindow};
use humanity_core::{SiteKind, Tribe};
use std::collections::BTreeMap;

#[derive(Clone, Copy, Default, PartialEq, Eq)]
pub enum Tab {
    #[default]
    Bands,
    Camps,
    Cultures,
    Languages,
}
impl Tab {
    const ALL: [Self; 4] = [Self::Bands, Self::Camps, Self::Cultures, Self::Languages];
    fn label(self) -> &'static str {
        match self {
            Self::Bands => "Bands",
            Self::Camps => "Camps",
            Self::Cultures => "Cultures",
            Self::Languages => "Languages",
        }
    }
}

#[derive(Clone, Copy, PartialEq, Eq)]
enum Key {
    Band(u32),
    Camp(u64),
    Culture(u32),
    Language(u32),
}
#[derive(Clone)]
struct Row {
    key: Key,
    label: String,
}
#[derive(Clone, Copy)]
enum Filter {
    Culture(u32),
    Language(u32),
    Camp { cell: CellId, founder: u32 },
}

pub struct Navigator {
    pub open: bool,
    tab: Tab,
    filter: Option<Filter>,
    back: Option<Tab>,
    rows: Vec<Row>,
    cursor: usize,
    page_size: usize,
    refresh: bool,
    last_day: u64,
    revision: u64,
    last_update: f64,
}
impl Default for Navigator {
    fn default() -> Self {
        Self {
            open: true,
            tab: Tab::Bands,
            filter: None,
            back: None,
            rows: Vec::new(),
            cursor: 0,
            page_size: 8,
            refresh: true,
            last_day: u64::MAX,
            revision: u64::MAX,
            last_update: -1.,
        }
    }
}

#[derive(Component)]
pub struct Panel;
#[derive(Component, Clone, Copy)]
pub enum Action {
    Toggle,
    Tab(Tab),
    Row(usize),
    Previous,
    Next,
    Back,
}

fn band_matches(t: &Tribe, filter: Option<Filter>) -> bool {
    match filter {
        None => true,
        Some(Filter::Culture(id)) => t.culture == id,
        Some(Filter::Language(id)) => t.languages.iter().any(|l| l.family == id && l.share > 0.),
        Some(Filter::Camp { cell, founder }) => t.cell == cell || t.id == founder,
    }
}

fn rows(world: &World, tab: Tab, filter: Option<Filter>) -> Vec<Row> {
    match tab {
        Tab::Bands => {
            let mut bands: Vec<_> = world
                .tribes
                .iter()
                .filter(|t| t.headcount() > 0 && band_matches(t, filter))
                .collect();
            bands.sort_by_key(|t| t.id);
            bands
                .into_iter()
                .map(|t| {
                    let suffix = match filter {
                        Some(Filter::Camp { cell, founder })
                            if t.id == founder && t.cell != cell =>
                        {
                            " / founder elsewhere"
                        }
                        _ => "",
                    };
                    Row {
                        key: Key::Band(t.id),
                        label: format!(
                            "Band {} / {} people{}\nC{} / cell {} / health {:.0}%",
                            t.id,
                            t.headcount(),
                            suffix,
                            t.culture,
                            t.cell,
                            t.health * 100.
                        ),
                    }
                })
                .collect()
        }
        Tab::Camps => {
            let mut population = BTreeMap::<CellId, u64>::new();
            for t in &world.tribes {
                *population.entry(t.cell).or_default() += u64::from(t.headcount());
            }
            // Present one culture/place entry for the legacy per-season records.
            let mut places = BTreeMap::new();
            for s in &world.sites {
                let entry = places
                    .entry((s.culture, s.cell, s.kind as u8))
                    .or_insert((s.id, 0_u8));
                entry.0 = entry.0.min(s.id);
                entry.1 |= 1 << s.season;
            }
            places
                .into_iter()
                .map(|((culture, cell, kind), (id, seasons))| {
                    let use_label = if kind == SiteKind::FishingSettlement as u8 {
                        "Fishing".into()
                    } else {
                        ["Spr", "Sum", "Aut", "Win"]
                            .into_iter()
                            .enumerate()
                            .filter(|(i, _)| seasons & (1 << i) != 0)
                            .map(|(_, s)| s)
                            .collect::<Vec<_>>()
                            .join("/")
                    };
                    Row {
                        key: Key::Camp(id),
                        label: format!(
                            "Camp {id} / C{culture} / cell {cell}\n{use_label} / {} people in cell",
                            population.get(&cell).copied().unwrap_or(0)
                        ),
                    }
                })
                .collect()
        }
        Tab::Cultures => {
            let mut counts = BTreeMap::<u32, (u64, usize)>::new();
            for t in &world.tribes {
                let entry = counts.entry(t.culture).or_default();
                entry.0 += u64::from(t.headcount());
                entry.1 += 1;
            }
            counts
                .into_iter()
                .map(|(id, (people, bands))| {
                    let c = &world.cultures[id as usize];
                    Row {
                        key: Key::Culture(id),
                        label: format!(
                            "C{id} / {} / {people} people\n{} / {bands} bands",
                            c.name,
                            c.variant.label()
                        ),
                    }
                })
                .collect()
        }
        Tab::Languages => {
            let mut counts = BTreeMap::<u32, (f64, usize)>::new();
            for t in &world.tribes {
                for language in &t.languages {
                    if language.share > 0. {
                        let entry = counts.entry(language.family).or_default();
                        entry.0 += f64::from(t.headcount()) * f64::from(language.share);
                        entry.1 += 1;
                    }
                }
            }
            counts
                .into_iter()
                .map(|(id, (people, bands))| Row {
                    key: Key::Language(id),
                    label: format!(
                        "Language family {id}\n~{people:.0} people (weighted) / {bands} bands"
                    ),
                })
                .collect()
        }
    }
}

fn refresh(world: &World, nav: &mut Navigator) {
    let selected = nav.rows.get(nav.cursor).map(|r| r.key);
    nav.rows = rows(world, nav.tab, nav.filter);
    nav.cursor = selected
        .and_then(|id| nav.rows.iter().position(|r| r.key == id))
        .unwrap_or(nav.cursor.min(nav.rows.len().saturating_sub(1)));
    nav.last_day = world.day;
}

fn focus(world: &World, view: &mut ViewState, cell: CellId) {
    view.selected = Some(cell);
    view.following = None;
    view.orbit = (Quat::from_rotation_arc(view.orbit * Vec3::Z, world.grid.direction(cell))
        * view.orbit)
        .normalize();
    view.distance = PLANET_RADIUS
        + (map::cell_spacing(world) * 12.).clamp(PLANET_RADIUS * 0.025, PLANET_RADIUS * 1.4);
    view.drag_start = None;
    view.dirty = true;
}

fn activate(world: &World, view: &mut ViewState, key: Key) {
    match key {
        Key::Band(id) => {
            if let Some((i, t)) = world.tribes.iter().enumerate().find(|(_, t)| t.id == id) {
                focus(world, view, t.cell);
                view.following = Some(id);
                view.focus_index = i;
            } else {
                view.status = format!("Band {id} is no longer alive.");
            }
        }
        Key::Camp(id) => {
            if let Some(site) = world.sites.iter().find(|s| s.id == id) {
                focus(world, view, site.cell);
                view.status = format!(
                    "{} / select a resident or founder band in the browser.",
                    site.name
                );
                view.navigation.filter = Some(Filter::Camp {
                    cell: site.cell,
                    founder: site.founder_band,
                });
                view.navigation.back = Some(Tab::Camps);
                view.navigation.tab = Tab::Bands;
            } else {
                view.status = "This camp is no longer remembered.".into();
            }
        }
        Key::Culture(id) => {
            view.navigation.filter = Some(Filter::Culture(id));
            view.navigation.back = Some(Tab::Cultures);
            view.navigation.tab = Tab::Bands;
        }
        Key::Language(id) => {
            view.navigation.filter = Some(Filter::Language(id));
            view.navigation.back = Some(Tab::Languages);
            view.navigation.tab = Tab::Bands;
        }
    }
    if !matches!(key, Key::Band(_)) {
        view.navigation.rows.clear();
        view.navigation.cursor = 0;
    }
    view.navigation.refresh = true;
}

fn action(world: &World, view: &mut ViewState, command: Action) {
    match command {
        Action::Toggle => {
            view.navigation.open = !view.navigation.open;
            view.drag_start = None;
        }
        Action::Tab(tab) => {
            let nav = &mut view.navigation;
            nav.tab = tab;
            nav.filter = None;
            nav.back = None;
            nav.cursor = 0;
            nav.rows.clear();
        }
        Action::Row(index) => {
            if let Some(row) = view.navigation.rows.get(index) {
                let key = row.key;
                view.navigation.cursor = index;
                activate(world, view, key);
            }
        }
        Action::Previous => {
            let nav = &mut view.navigation;
            nav.cursor = nav.cursor.saturating_sub(nav.page_size);
        }
        Action::Next => {
            let nav = &mut view.navigation;
            nav.cursor = (nav.cursor + nav.page_size).min(nav.rows.len().saturating_sub(1));
        }
        Action::Back => {
            let tab = view.navigation.back.unwrap_or(Tab::Bands);
            action(world, view, Action::Tab(tab));
        }
    }
    view.navigation.refresh = true;
}

/// Alt+arrows own their key presses; plain arrows remain globe controls.
pub fn keyboard(keys: &ButtonInput<KeyCode>, world: &World, view: &mut ViewState) {
    if keys.just_pressed(KeyCode::KeyB) {
        action(world, view, Action::Toggle);
    }
    let alt = keys.any_pressed([KeyCode::AltLeft, KeyCode::AltRight]);
    if alt
        && keys.any_just_pressed([
            KeyCode::ArrowLeft,
            KeyCode::ArrowRight,
            KeyCode::ArrowUp,
            KeyCode::ArrowDown,
        ])
    {
        view.navigation.open = true;
        if keys.any_just_pressed([KeyCode::ArrowLeft, KeyCode::ArrowRight]) {
            let step = if keys.just_pressed(KeyCode::ArrowRight) {
                1
            } else {
                3
            };
            let tab = Tab::ALL[(view.navigation.tab as usize + step) % Tab::ALL.len()];
            action(world, view, Action::Tab(tab));
        }
        refresh(world, &mut view.navigation);
        let nav = &mut view.navigation;
        if keys.just_pressed(KeyCode::ArrowDown) {
            nav.cursor = (nav.cursor + 1).min(nav.rows.len().saturating_sub(1));
        }
        if keys.just_pressed(KeyCode::ArrowUp) {
            nav.cursor = nav.cursor.saturating_sub(1);
        }
        nav.refresh = true;
    }
    if view.navigation.open {
        if keys.any_just_pressed([KeyCode::Enter, KeyCode::NumpadEnter]) {
            action(world, view, Action::Row(view.navigation.cursor));
        }
        if keys.just_pressed(KeyCode::Backspace) {
            action(world, view, Action::Back);
        }
        if keys.just_pressed(KeyCode::PageUp) {
            action(world, view, Action::Previous);
        }
        if keys.just_pressed(KeyCode::PageDown) {
            action(world, view, Action::Next);
        }
    }
}

pub fn blocks_map(view: &ViewState, p: Vec2, height: f32) -> bool {
    p.x < 350. && p.y > 100. && p.y < height - 72. && view.navigation.open
        || (p.x < 180. && p.y > 92. && p.y < 132.)
}

pub fn interact(
    game: Res<Game>,
    mut view: ResMut<ViewState>,
    buttons: Query<(&Interaction, &Action), Changed<Interaction>>,
    mut wheel: MessageReader<MouseWheel>,
    window: Single<&Window, With<PrimaryWindow>>,
) {
    if view.insights_open {
        wheel.clear();
        return;
    }
    for (interaction, a) in &buttons {
        if *interaction == Interaction::Pressed {
            action(&game.world, &mut view, *a);
        }
    }
    for event in wheel.read() {
        if view.navigation.open
            && window
                .cursor_position()
                .is_some_and(|p| blocks_map(&view, p, window.height()))
        {
            let nav = &mut view.navigation;
            if event.y > 0. {
                nav.cursor = nav.cursor.saturating_sub(1);
            }
            if event.y < 0. {
                nav.cursor = (nav.cursor + 1).min(nav.rows.len().saturating_sub(1));
            }
            nav.refresh = true;
        }
    }
}

fn button(parent: &mut ChildSpawnerCommands, label: &str, action: Action, selected: bool) {
    parent
        .spawn((
            Button,
            action,
            Node {
                padding: UiRect::all(px(7)),
                min_height: px(32),
                ..default()
            },
            BackgroundColor(if selected {
                Color::srgb(0.22, 0.31, 0.32)
            } else {
                Color::srgb(0.08, 0.13, 0.15)
            }),
        ))
        .with_children(|p| {
            p.spawn((
                Text::new(label),
                TextFont {
                    font_size: 13.,
                    ..default()
                },
                TextColor(Color::srgb(0.9, 0.88, 0.75)),
            ));
        });
}

pub fn update(
    mut commands: Commands,
    game: Res<Game>,
    mut view: ResMut<ViewState>,
    time: Res<Time>,
    window: Single<&Window, With<PrimaryWindow>>,
    panels: Query<Entity, With<Panel>>,
) {
    let revision = view.snapshot_revision;
    let insights = view.insights_open;
    let page_size = ((window.height() - 350.) / 64.).floor().clamp(1., 12.) as usize;
    let nav = &mut view.navigation;
    if nav.revision != revision {
        *nav = Navigator {
            open: nav.open,
            revision,
            ..default()
        };
    }
    if nav.page_size != page_size {
        nav.page_size = page_size;
        nav.refresh = true;
    }
    if insights {
        for entity in &panels {
            commands.entity(entity).despawn();
        }
        nav.refresh = true;
        return;
    }
    if !nav.refresh
        && (nav.last_day == game.world.day || time.elapsed_secs_f64() - nav.last_update < 0.25)
    {
        return;
    }
    refresh(&game.world, nav);
    nav.refresh = false;
    nav.last_update = time.elapsed_secs_f64();
    for entity in &panels {
        commands.entity(entity).despawn();
    }
    commands
        .spawn((
            Panel,
            Node {
                position_type: PositionType::Absolute,
                left: px(20),
                top: px(102),
                width: px(if nav.open { 310. } else { 140. }),
                max_height: px((window.height() - 180.).max(80.)),
                padding: UiRect::all(px(10)),
                flex_direction: FlexDirection::Column,
                row_gap: px(7),
                overflow: Overflow::clip(),
                ..default()
            },
            BackgroundColor(Color::srgba(0.035, 0.065, 0.08, 0.97)),
        ))
        .with_children(|p| {
            button(
                p,
                if nav.open {
                    "BROWSER / B to hide"
                } else {
                    "B / Browse"
                },
                Action::Toggle,
                false,
            );
            if !nav.open {
                return;
            }
            p.spawn(Node {
                flex_direction: FlexDirection::Row,
                column_gap: px(3),
                ..default()
            })
            .with_children(|p| {
                for tab in Tab::ALL {
                    button(p, tab.label(), Action::Tab(tab), nav.tab == tab);
                }
            });
            let context = match nav.filter {
                None => format!("{} / {} entries", nav.tab.label(), nav.rows.len()),
                Some(Filter::Culture(id)) => format!("Culture {id} / {} bands", nav.rows.len()),
                Some(Filter::Language(id)) => format!("Family {id} / {} bands", nav.rows.len()),
                Some(Filter::Camp { cell, .. }) => format!("Cell {cell} / residents + founder"),
            };
            p.spawn((
                Text::new(context),
                TextFont {
                    font_size: 14.,
                    ..default()
                },
            ));
            if nav.back.is_some() {
                button(p, "< Back / Backspace", Action::Back, false);
            }
            let start = nav.cursor / nav.page_size * nav.page_size;
            for (index, row) in nav.rows.iter().enumerate().skip(start).take(nav.page_size) {
                button(p, &row.label, Action::Row(index), index == nav.cursor);
            }
            if nav.rows.is_empty() {
                p.spawn((
                    Text::new("No living bands / remembered places."),
                    TextFont {
                        font_size: 13.,
                        ..default()
                    },
                ));
            }
            p.spawn(Node {
                flex_direction: FlexDirection::Row,
                column_gap: px(8),
                ..default()
            })
            .with_children(|p| {
                button(p, "< Previous", Action::Previous, false);
                button(
                    p,
                    &format!(
                        "{} / {}",
                        start / nav.page_size + 1,
                        nav.rows.len().div_ceil(nav.page_size).max(1)
                    ),
                    Action::Next,
                    false,
                );
                button(p, "Next >", Action::Next, false);
            });
            p.spawn((
                Text::new(
                    "Alt+Up/Down select; Left/Right category\nEnter follow/open; wheel / PgUp/PgDn",
                ),
                TextFont {
                    font_size: 11.,
                    ..default()
                },
                TextColor(Color::srgb(0.65, 0.76, 0.75)),
            ));
        });
}

#[cfg(test)]
mod tests {
    use super::*;
    use bevy::ecs::system::RunSystemOnce;
    use humanity_core::{CulturalSite, Language, PlanetConfig};

    fn world() -> World {
        humanity_world::generate(PlanetConfig {
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
    fn camp_groups_drill_into_residents_and_absent_founder_then_follow_stable_id() {
        let mut w = world();
        let founder = w.tribes[0].id;
        let cell = w.tribes[1].cell;
        // Ensure the founder has moved elsewhere; it must still be reachable.
        w.tribes[0].cell = w.grid.neighbors(cell).next().unwrap();
        for season in 0..4 {
            w.sites.push(CulturalSite {
                id: u64::from(season),
                name: format!("Camp {season}"),
                cell,
                culture: w.tribes[0].culture,
                founder_band: founder,
                kind: SiteKind::SeasonalCamp,
                season,
                founded: 0,
                last_visited: 0,
            });
        }
        let mut view = ViewState::default();
        action(&w, &mut view, Action::Tab(Tab::Camps));
        refresh(&w, &mut view.navigation);
        assert_eq!(view.navigation.rows.len(), 1);
        action(&w, &mut view, Action::Row(0));
        assert_eq!(view.selected, Some(cell));
        refresh(&w, &mut view.navigation);
        let index = view
            .navigation
            .rows
            .iter()
            .position(|r| r.key == Key::Band(founder))
            .unwrap();
        assert!(
            view.navigation.rows[index]
                .label
                .contains("founder elsewhere")
        );
        action(&w, &mut view, Action::Row(index));
        assert_eq!(view.following, Some(founder));
        assert_eq!(view.selected, Some(w.tribes[0].cell));
        action(&w, &mut view, Action::Back);
        assert!(view.navigation.tab == Tab::Camps && view.navigation.filter.is_none());
    }

    #[test]
    fn culture_and_language_counts_and_cursor_survive_membership_changes() {
        let mut w = world();
        for t in &mut w.tribes {
            t.culture = 0;
            t.languages = vec![
                Language {
                    family: 7,
                    share: 0.75,
                },
                Language {
                    family: 42,
                    share: 0.25,
                },
            ];
        }
        let cultures = rows(&w, Tab::Cultures, None);
        assert_eq!(cultures.len(), 1);
        assert!(cultures[0].label.contains("500 people"));
        let languages = rows(&w, Tab::Languages, None);
        assert_eq!(languages.len(), 2);
        assert!(languages[0].label.contains("~375 people"));
        assert!(languages[1].label.contains("~125 people"));
        let mut view = ViewState::default();
        activate(&w, &mut view, Key::Language(42));
        refresh(&w, &mut view.navigation);
        assert_eq!(view.navigation.rows.len(), w.tribes.len());
        view.navigation.cursor = 1;
        let selected = view.navigation.rows[1].key;
        w.tribes.remove(0);
        refresh(&w, &mut view.navigation);
        assert!(view.navigation.rows[view.navigation.cursor].key == selected);
        assert_eq!(view.navigation.cursor, 0);
    }

    #[test]
    fn mouse_buttons_and_keyboard_activate_the_same_band_navigation() {
        let w = world();
        let mut view = ViewState::default();
        refresh(&w, &mut view.navigation);
        let expected = w.tribes[1].id;
        let mut keys = ButtonInput::default();
        keys.press(KeyCode::AltLeft);
        keys.press(KeyCode::ArrowDown);
        keyboard(&keys, &w, &mut view);
        assert_eq!(view.navigation.cursor, 1);
        keys.reset_all();
        keys.press(KeyCode::Enter);
        keyboard(&keys, &w, &mut view);
        assert_eq!(view.following, Some(expected));
        view.following = None;
        let mut app = App::new();
        app.insert_resource(Game {
            world: w,
            simulation: Simulation::default(),
        })
        .insert_resource(view)
        .add_message::<MouseWheel>();
        app.world_mut().spawn((Window::default(), PrimaryWindow));
        app.world_mut()
            .spawn((Interaction::Pressed, Action::Row(1)));
        app.world_mut().run_system_once(interact).unwrap();
        assert_eq!(
            app.world().resource::<ViewState>().following,
            Some(expected)
        );
    }
}
