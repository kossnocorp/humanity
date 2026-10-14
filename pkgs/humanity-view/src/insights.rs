//! Population diagnostics are built from persistent simulation telemetry, not the
//! short general-purpose event feed. The overlay remains live as time advances.
use crate::*;
use bevy::input::mouse::{MouseScrollUnit, MouseWheel};
use humanity_core::{DeathCause, PopulationSample};

#[derive(Component)]
pub struct Dashboard;

const RANGES: [(&str, u64, bool); 9] = [
    ("30 days", 30, false),
    ("90 days", 90, false),
    ("180 days", 180, false),
    ("1 year", 1, true),
    ("3 years", 3, true),
    ("5 years", 5, true),
    ("10 years", 10, true),
    ("25 years", 25, true),
    ("100 years", 100, true),
];

#[derive(Component)]
pub struct RangeButton(usize);

pub fn interact(
    buttons: Query<(&Interaction, &RangeButton), Changed<Interaction>>,
    mut view: ResMut<ViewState>,
) {
    for (interaction, range) in &buttons {
        if *interaction == Interaction::Pressed {
            view.chart_range = range.0;
        }
    }
}

fn selected_history(world: &World, range: usize) -> Vec<&PopulationSample> {
    let (_, length, years) = RANGES[range];
    let days = if years {
        length * u64::from(world.config.year_days())
    } else {
        length
    };
    let start = if length == 0 {
        0
    } else {
        world.day.saturating_sub(days)
    };
    world
        .insights
        .history
        .iter()
        .filter(|s| s.day >= start)
        .collect()
}
const TEXT: Color = Color::srgb(0.87, 0.9, 0.85);
const MUTED: Color = Color::srgb(0.57, 0.68, 0.7);
const GOLD: Color = Color::srgb(0.95, 0.78, 0.49);
const GREEN: Color = Color::srgb(0.44, 0.77, 0.60);
const RED: Color = Color::srgb(0.91, 0.42, 0.34);

fn label(
    commands: &mut Commands,
    parent: Entity,
    text: impl Into<String>,
    size: f32,
    color: Color,
) {
    let child = commands
        .spawn((
            Text::new(text),
            TextFont {
                font_size: size,
                ..default()
            },
            TextColor(color),
        ))
        .id();
    commands.entity(parent).add_child(child);
}

fn panel(commands: &mut Commands, parent: Entity) -> Entity {
    let id = commands
        .spawn((
            Node {
                flex_direction: FlexDirection::Column,
                flex_grow: 1.,
                flex_basis: px(0),
                flex_shrink: 0.,
                min_width: px(0),
                padding: UiRect::all(px(16)),
                row_gap: px(7),
                ..default()
            },
            BackgroundColor(Color::srgb(0.065, 0.105, 0.13)),
        ))
        .id();
    commands.entity(parent).add_child(id);
    id
}

fn row(commands: &mut Commands, parent: Entity) -> Entity {
    let id = commands
        .spawn(Node {
            column_gap: px(14),
            width: percent(100),
            flex_shrink: 0.,
            ..default()
        })
        .id();
    commands.entity(parent).add_child(id);
    id
}

fn bar(commands: &mut Commands, parent: Entity, width: f32, color: Color) {
    let track = commands
        .spawn((
            Node {
                height: px(8),
                width: percent(100),
                ..default()
            },
            BackgroundColor(Color::srgb(0.12, 0.17, 0.19)),
        ))
        .id();
    let fill = commands
        .spawn((
            Node {
                height: percent(100),
                width: percent(width.clamp(0., 100.)),
                ..default()
            },
            BackgroundColor(color),
        ))
        .id();
    commands.entity(track).add_child(fill);
    commands.entity(parent).add_child(track);
}

fn plot(
    commands: &mut Commands,
    parent: Entity,
    values: &[(f32, f32)],
    max: f32,
    colors: [Color; 2],
) {
    let plot = commands
        .spawn((
            Node {
                height: px(125),
                width: percent(100),
                align_items: AlignItems::End,
                column_gap: px(1),
                ..default()
            },
            BackgroundColor(Color::srgb(0.035, 0.065, 0.08)),
        ))
        .id();
    commands.entity(parent).add_child(plot);
    for &(a, b) in values {
        let group = commands
            .spawn(Node {
                height: percent(100),
                flex_grow: 1.,
                flex_basis: px(0),
                align_items: AlignItems::End,
                ..default()
            })
            .id();
        commands.entity(plot).add_child(group);
        for (value, color) in [(a, colors[0]), (b, colors[1])] {
            let rect = commands
                .spawn((
                    Node {
                        height: percent((value / max.max(1.) * 100.).clamp(0., 100.)),
                        flex_grow: 1.,
                        ..default()
                    },
                    BackgroundColor(color),
                ))
                .id();
            commands.entity(group).add_child(rect);
        }
    }
}

fn cause_color(c: DeathCause) -> Color {
    match c {
        DeathCause::Nutrition => RED,
        DeathCause::Background => MUTED,
        DeathCause::Aging => GOLD,
        DeathCause::Conflict => Color::srgb(0.68, 0.49, 0.81),
    }
}

#[derive(Default)]
pub struct DashboardCache {
    key: Option<(u64, u64, bool, u32, usize)>,
    updated: f64,
    snapshot_revision: u64,
}

pub fn update(
    mut commands: Commands,
    game: Res<Game>,
    view: Res<ViewState>,
    roots: Query<(Entity, &ScrollPosition), With<Dashboard>>,
    time: Res<Time>,
    mut cache: Local<DashboardCache>,
) {
    if !view.insights_open {
        for (root, _) in &roots {
            commands.entity(root).despawn();
        }
        cache.key = None;
        return;
    }
    let w = &game.world;
    let d = &w.insights;
    let key = (
        w.day,
        w.rng.state,
        view.paused,
        view.speed.to_bits(),
        view.chart_range,
    );
    if !roots.is_empty()
        && ((cache.key == Some(key) && cache.snapshot_revision == view.snapshot_revision)
            || (cache.key.is_some_and(|k| k.4 == view.chart_range)
                && time.elapsed_secs_f64() - cache.updated < 0.25))
    {
        return;
    }
    cache.key = Some(key);
    cache.snapshot_revision = view.snapshot_revision;
    cache.updated = time.elapsed_secs_f64();
    let mut offset = Vec2::ZERO;
    for (root, scroll) in &roots {
        offset = scroll.0;
        commands.entity(root).despawn();
    }
    let root = commands
        .spawn((
            Dashboard,
            GlobalZIndex(100),
            Node {
                position_type: PositionType::Absolute,
                left: px(0),
                top: px(0),
                width: percent(100),
                height: percent(100),
                padding: UiRect::all(px(24)),
                flex_direction: FlexDirection::Column,
                row_gap: px(12),
                overflow: Overflow::scroll_y(),
                ..default()
            },
            ScrollPosition(offset),
            BackgroundColor(Color::srgb(0.028, 0.05, 0.068)),
        ))
        .id();
    label(
        &mut commands,
        root,
        "P O P U L A T I O N   /   WHY PEOPLE DIE",
        24.,
        GOLD,
    );
    label(
        &mut commands,
        root,
        format!(
            "Day {} | {} people | {} births - {} deaths = {:+} net | {} {:.2} days/s | SPACE pause  -/+ speed  ?/Esc close",
            w.day,
            w.population(),
            w.totals.births,
            w.totals.deaths,
            w.population() as i64 - i64::from(w.config.population),
            if view.paused { "PAUSED" } else { "LIVE" },
            view.speed
        ),
        14.,
        TEXT,
    );
    let switches = row(&mut commands, root);
    commands.entity(switches).insert(Node {
        flex_wrap: FlexWrap::Wrap,
        column_gap: px(6),
        row_gap: px(6),
        flex_shrink: 0.,
        ..default()
    });
    for (index, (text, _, _)) in RANGES.iter().enumerate() {
        let button = commands
            .spawn((
                Button,
                RangeButton(index),
                Node {
                    padding: UiRect::axes(px(10), px(8)),
                    ..default()
                },
                BackgroundColor(if index == view.chart_range {
                    Color::srgb(0.25, 0.35, 0.38)
                } else {
                    Color::srgb(0.09, 0.14, 0.17)
                }),
            ))
            .id();
        commands.entity(switches).add_child(button);
        label(
            &mut commands,
            button,
            *text,
            13.,
            if index == view.chart_range {
                GOLD
            } else {
                TEXT
            },
        );
    }
    let Some(latest) = d.history.back() else {
        label(
            &mut commands,
            root,
            "No recorded population history yet. Advance the simulation to collect samples.",
            16.,
            TEXT,
        );
        return;
    };
    let history = selected_history(w, view.chart_range);
    let first = history[0];
    let recent = first;
    label(
        &mut commands,
        root,
        format!(
            "Last {} recorded days: +{} births / -{} deaths   |   {:.0}% mean health   |   {} people below 50% health   |   {} with <1 day food",
            w.day - recent.day,
            latest.births - recent.births,
            latest.deaths.checked_total().unwrap_or(0) - recent.deaths.checked_total().unwrap_or(0),
            latest.mean_health * 100.,
            latest.low_health,
            latest.low_food
        ),
        13.,
        MUTED,
    );

    let charts = row(&mut commands, root);
    let population = panel(&mut commands, charts);
    let minimum = history.iter().map(|s| s.population).min().unwrap_or(0);
    let maximum = history.iter().map(|s| s.population).max().unwrap_or(0);
    label(&mut commands, population, "POPULATION OVER TIME", 15., GOLD);
    label(
        &mut commands,
        population,
        format!(
            "{} .. {} people (truncated vertical axis)",
            minimum,
            maximum.max(minimum + 1)
        ),
        12.,
        MUTED,
    );
    let stride = history.len().div_ceil(96).max(1);
    let values: Vec<_> = history
        .chunks(stride)
        .map(|chunk| {
            let value = (chunk.last().unwrap().population - minimum) as f32;
            (value, value)
        })
        .collect();
    plot(
        &mut commands,
        population,
        &values,
        (maximum - minimum).max(1) as f32,
        [GREEN; 2],
    );
    label(
        &mut commands,
        population,
        format!(
            "Day {} -> {}   |   {} daily samples retained",
            first.day,
            w.day,
            history.len()
        ),
        12.,
        MUTED,
    );

    let flows = panel(&mut commands, charts);
    label(&mut commands, flows, "BIRTHS & DEATHS OVER TIME", 15., GOLD);
    let intervals = flow_buckets(&history, stride);
    let peak = intervals
        .iter()
        .map(|&(a, b)| a.max(b))
        .fold(1_f32, f32::max);
    label(
        &mut commands,
        flows,
        format!("Green births / red deaths   |   0 .. {peak:.0} per bucket"),
        12.,
        MUTED,
    );
    plot(&mut commands, flows, &intervals, peak, [GREEN, RED]);
    label(
        &mut commands,
        flows,
        format!(
            "Day {} -> {}   |   up to {} days per bucket",
            first.day, w.day, stride
        ),
        12.,
        MUTED,
    );

    let middle = row(&mut commands, root);
    let causes = panel(&mut commands, middle);
    label(
        &mut commands,
        causes,
        "DEATHS BY CAUSE / SELECTED RANGE",
        15.,
        GOLD,
    );
    let total =
        latest.deaths.checked_total().unwrap_or(0) - first.deaths.checked_total().unwrap_or(0);
    for c in DeathCause::ALL {
        let count = latest.deaths.get(c) - first.deaths.get(c);
        let share = count as f32 / total.max(1) as f32 * 100.;
        label(
            &mut commands,
            causes,
            format!("{}   {} ({share:.1}%)", c.label(), count),
            12.,
            cause_color(c),
        );
        bar(&mut commands, causes, share, cause_color(c));
    }
    let cause_charts = row(&mut commands, root);
    commands.entity(cause_charts).insert(Node {
        flex_wrap: FlexWrap::Wrap,
        column_gap: px(14),
        row_gap: px(14),
        flex_shrink: 0.,
        ..default()
    });
    let series: Vec<_> = DeathCause::ALL
        .into_iter()
        .map(|cause| {
            let mut previous = first;
            let values: Vec<_> = history[1..]
                .chunks(stride)
                .map(|chunk| {
                    let current = *chunk.last().unwrap();
                    let count = (current.deaths.get(cause) - previous.deaths.get(cause)) as f32;
                    let rate = count / (current.day - previous.day).max(1) as f32;
                    previous = current;
                    (rate, rate)
                })
                .collect();
            (cause, values)
        })
        .collect();
    let peak = series
        .iter()
        .flat_map(|(_, values)| values.iter().map(|v| v.0))
        .fold(1_f32, f32::max);
    for (cause, values) in series {
        let chart = panel(&mut commands, cause_charts);
        commands.entity(chart).insert(Node {
            flex_direction: FlexDirection::Column,
            width: px(260),
            flex_grow: 1.,
            padding: UiRect::all(px(16)),
            row_gap: px(7),
            ..default()
        });
        label(&mut commands, chart, cause.label(), 14., cause_color(cause));
        plot(&mut commands, chart, &values, peak, [cause_color(cause); 2]);
        label(
            &mut commands,
            chart,
            format!(
                "0 .. {peak:.2} deaths/day (bucket mean)\nDay {} -> {}",
                first.day, latest.day
            ),
            12.,
            MUTED,
        );
    }
    let cohorts = panel(&mut commands, middle);
    label(
        &mut commands,
        cohorts,
        "WHO IS ALIVE / CURRENT VULNERABILITY",
        15.,
        GOLD,
    );
    for (name, n) in ["Children <15", "Adults 15-49", "Elders 50+"]
        .into_iter()
        .zip(latest.cohorts)
    {
        label(&mut commands, cohorts, format!("{name}   {n}"), 12., TEXT);
        bar(
            &mut commands,
            cohorts,
            n as f32 / latest.population.max(1) as f32 * 100.,
            GREEN,
        );
    }
    label(
        &mut commands,
        cohorts,
        "Food shortages reduce health; poor health raises mortality.\nSnow limits gathering and travel; cold increases food needs.\nConflicts kill directly and also reduce survivors' health.\nRisk counts describe people, not the number of bands.",
        12.,
        MUTED,
    );

    let bottom = row(&mut commands, root);
    let records = panel(&mut commands, bottom);
    label(
        &mut commands,
        records,
        "RECENT DEATHS / CONTEXT AT RESOLUTION",
        15.,
        GOLD,
    );
    for r in d.recent_deaths.iter().rev().take(4) {
        label(
            &mut commands,
            records,
            format!(
                "D{} / Band {} / {} x {}\n  Health {:.0}% | food {:.1}d | {:.0} C | snow {:.0}%",
                r.day,
                r.tribe,
                r.count,
                r.cause.label(),
                r.health * 100.,
                r.food_days,
                r.temperature,
                r.snow * 100.
            ),
            12.,
            cause_color(r.cause),
        );
    }
    if d.recent_deaths.is_empty() {
        label(
            &mut commands,
            records,
            "No deaths recorded since diagnostics began.",
            13.,
            MUTED,
        );
    }
    let explanation = panel(&mut commands, bottom);
    label(
        &mut commands,
        explanation,
        "HOW TO READ THE CAUSES",
        15.,
        GOLD,
    );
    label(
        &mut commands,
        explanation,
        "Nutrition / poor health: extra daily risk = 2.5% x (1-health)^3.\nBackground: 1.8% per simulated year, independent of health.\nThese share a mortality roll; causes are sampled in proportion\nto their risks, without changing the simulation's random stream.\nElders: additional 8% annual risk for the 50+ cohorts.\nConflict: actual casualties from encounter resolution.\nWeather is context, not a separate direct cause of death.",
        12.,
        TEXT,
    );
    label(
        &mut commands,
        explanation,
        format!(
            "Tracking began on day {}. Up to {} daily samples / {} death groups kept.\nCharts show available history only; years use this planet's calendar.",
            d.since_day.unwrap_or(w.day),
            humanity_core::history_capacity(w.config.year_days()),
            humanity_core::DEATH_RECORDS
        ),
        12.,
        MUTED,
    );
}

pub fn scroll(
    mut wheel: MessageReader<MouseWheel>,
    mut roots: Query<&mut ScrollPosition, With<Dashboard>>,
) {
    for e in wheel.read() {
        let delta = match e.unit {
            MouseScrollUnit::Line => e.y * 32.,
            MouseScrollUnit::Pixel => e.y,
        };
        for mut position in &mut roots {
            position.0.y = (position.0.y - delta).max(0.);
        }
    }
}

fn flow_buckets(history: &[&PopulationSample], stride: usize) -> Vec<(f32, f32)> {
    if history.is_empty() {
        return Vec::new();
    }
    let mut previous = history[0];
    history[1..]
        .chunks(stride)
        .map(|chunk| {
            let current = *chunk.last().unwrap();
            let result = (
                (current.births - previous.births) as f32,
                (current.deaths.checked_total().unwrap_or(0)
                    - previous.deaths.checked_total().unwrap_or(0)) as f32,
            );
            previous = current;
            result
        })
        .collect()
}

#[cfg(test)]
mod tests {
    use super::*;
    use bevy::ecs::system::RunSystemOnce;

    #[test]
    fn ranges_use_calendar_days_and_clicks_work_while_paused() {
        let mut world = humanity_world::generate(humanity_core::PlanetConfig {
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
        for day in 1..=12_100 {
            world.day = day;
            world.totals.deaths = day * 2;
            world.insights.deaths.nutrition = day * 2;
            world.record_population();
        }
        assert_eq!(world.insights.history.len(), 12_001);
        assert_eq!(world.insights.history.front().unwrap().day, 100);
        for (range, days) in [30, 90, 180, 120, 360, 600, 1200, 3000, 12000]
            .into_iter()
            .enumerate()
        {
            let history = selected_history(&world, range);
            assert_eq!(history.len(), days + 1);
            assert_eq!(
                history.last().unwrap().deaths.nutrition - history[0].deaths.nutrition,
                days as u64 * 2
            );
            let buckets = flow_buckets(&history, history.len().div_ceil(96));
            assert_eq!(buckets.iter().map(|b| b.1).sum::<f32>(), days as f32 * 2.);
        }
        let mut app = App::new();
        app.insert_resource(ViewState {
            paused: true,
            insights_open: true,
            ..default()
        });
        app.world_mut()
            .spawn((RangeButton(8), Interaction::Pressed));
        app.world_mut().run_system_once(interact).unwrap();
        assert_eq!(app.world().resource::<ViewState>().chart_range, 8);
        assert!(app.world().resource::<ViewState>().paused);
    }
}
