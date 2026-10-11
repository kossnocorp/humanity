use crate::*;
use humanity_core::EventKind;

#[derive(Component)]
pub enum Hud {
    Header,
    Notes,
    Chronicle,
    Status,
}

const INK: Color = Color::srgb(0.88, 0.88, 0.79);
const MUTED: Color = Color::srgb(0.57, 0.68, 0.68);
const GOLD: Color = Color::srgb(0.93, 0.79, 0.51);

pub fn setup(mut commands: Commands) {
    commands
        .spawn((
            Node {
                position_type: PositionType::Absolute,
                top: px(0),
                width: percent(100),
                height: px(90),
                padding: UiRect::all(px(24)),
                ..default()
            },
            BackgroundColor(Color::srgba(0.035, 0.065, 0.085, 0.98)),
        ))
        .with_children(|p| {
            p.spawn((Node {
                flex_direction: FlexDirection::Column,
                width: px(310),
                row_gap: px(5),
                ..default()
            },))
                .with_children(|p| {
                    p.spawn((
                        Text::new("H U M A N I T Y"),
                        TextFont {
                            font_size: 26.,
                            ..default()
                        },
                        TextColor(GOLD),
                    ));
                    p.spawn((
                        Text::new("BEFORE THE FIRST SETTLEMENTS"),
                        TextFont {
                            font_size: 11.,
                            ..default()
                        },
                        TextColor(MUTED),
                    ));
                });
            p.spawn((
                Hud::Header,
                Text::default(),
                TextFont {
                    font_size: 17.,
                    ..default()
                },
                TextColor(INK),
            ));
        });
    commands.spawn((Node { position_type: PositionType::Absolute, right: px(20), top: px(110), bottom: px(72),
        width: px(310), padding: UiRect::all(px(20)), flex_direction: FlexDirection::Column,
        row_gap: px(15), overflow: Overflow::clip(), ..default() },
        BackgroundColor(Color::srgba(0.045, 0.075, 0.09, 0.96))))
        .with_children(|p| {
            p.spawn((Text::new("F I E L D   N O T E S"), TextFont { font_size: 17., ..default() }, TextColor(GOLD)));
            p.spawn((Hud::Notes, Text::default(), TextFont { font_size: 14., ..default() }, TextColor(INK)));
            p.spawn((Text::new("RECENT ENCOUNTERS"), TextFont { font_size: 12., ..default() }, TextColor(GOLD)));
            p.spawn((Hud::Chronicle, Text::default(), TextFont { font_size: 12., ..default() }, TextColor(MUTED)));
            p.spawn((Text::new("Cream camps: human bands\nOchre animals: migrating herds\nBlue threads: rivers  /  Pale ground: snow"),
                TextFont { font_size: 11., ..default() }, TextColor(MUTED)));
        });
    commands.spawn((
        Hud::Status,
        Text::default(),
        TextFont {
            font_size: 13.,
            ..default()
        },
        TextColor(GOLD),
        Node {
            position_type: PositionType::Absolute,
            left: px(24),
            bottom: px(68),
            max_width: percent(70),
            ..default()
        },
    ));
    commands.spawn((Node { position_type: PositionType::Absolute, bottom: px(0), width: percent(100), height: px(50),
        padding: UiRect::axes(px(24), px(16)), ..default() }, BackgroundColor(Color::srgba(0.035, 0.065, 0.085, 0.98))))
        .with_children(|p| {
            p.spawn((Text::new("SPACE pause   - / + speed   ARROWS / DRAG pan   WHEEL zoom   CLICK inspect   S save   L load   TAB layer   F next band   HOME atlas   ? population"),
                TextFont { font_size: 13., ..default() }, TextColor(MUTED)));
        });
}

pub fn update(
    game: Res<Game>,
    view: Res<ViewState>,
    time: Res<Time>,
    mut last: Local<f64>,
    mut texts: Query<(&mut Text, &Hud)>,
) {
    if time.elapsed_secs_f64() - *last < 0.1 {
        return;
    }
    *last = time.elapsed_secs_f64();
    let w = &game.world;
    for (mut text, kind) in &mut texts {
        if matches!(kind, Hud::Header) {
            **text = format!(
                "Year {}  /  Day {}  /  {} in the north     {}  {:.2} days/s\n{} people in {} bands     {} animals in {} herds     {}",
                w.day / u64::from(w.config.year_days()) + 1,
                w.day,
                w.season(),
                if view.paused { "PAUSED" } else { "RUNNING" },
                view.speed,
                w.population(),
                w.tribes.len(),
                w.animals(),
                w.herds.len(),
                view.layer.label()
            );
        } else if matches!(kind, Hud::Notes) {
            **text = if let Some(id) = view.selected.filter(|&id| (id as usize) < w.cells.len()) {
                let c = &w.cells[id as usize];
                let people: Vec<_> = w.tribes.iter().filter(|t| t.cell == id).collect();
                let animals: u64 = w
                    .herds
                    .iter()
                    .filter(|h| h.cell == id)
                    .map(|h| u64::from(h.animals))
                    .sum();
                let mut body = format!(
                    "HEX {}, {}   /   {:?}\n{} m elevation  /  {:.0} km spacing\n{} latitude  /  river flow {:.0}\n\nWEATHER & LIVING LAND\n{:.1} C   /   rain {:.1} mm/day\nSnow {:.0}%   /   soil moisture {:.0}%\nPlants {:>7.0}   Fruit {:>7.0}\nGame {:>8.0}   Graze {:>6.0}\nFood: person-days; graze: animal-days\n{} herd animals here\n",
                    id % w.config.width,
                    id / w.config.width,
                    c.biome,
                    c.elevation,
                    w.config.cell_km,
                    if w.config.latitude(id) > 0. {
                        "Northern"
                    } else {
                        "Southern"
                    },
                    c.river,
                    c.weather.temperature,
                    c.weather.precipitation,
                    c.weather.snow * 100.,
                    c.weather.soil_moisture * 100.,
                    c.resources.plants,
                    c.resources.fruit,
                    c.resources.game,
                    c.resources.grazing,
                    animals
                );
                if let Some(t) = people.first() {
                    let languages = t
                        .languages
                        .iter()
                        .map(|l| format!("{}: {:.0}%", l.family, l.share * 100.))
                        .collect::<Vec<_>>()
                        .join("  ");
                    let children = t.people.female[0] + t.people.male[0];
                    let elders = t.people.female[2] + t.people.male[2];
                    body.push_str(&format!("\nBAND {}   /   {} people\nHealth {:.0}%  /  food {:.1} days\nFemale {}  /  Male {}\nYoung {}  /  Adult {}  /  Elder {}\nHunters {}  /  Gatherers {}\nHealers {}  /  Guides {}\nLanguage families\n{}\n{} band(s) share this hex",
                        t.id, t.headcount(), t.health * 100., t.food / t.headcount().max(1) as f32,
                        t.people.female.iter().sum::<u32>(), t.people.male.iter().sum::<u32>(), children, t.people.adults(), elders,
                        t.specialists.hunters, t.specialists.gatherers, t.specialists.healers, t.specialists.guides, languages, people.len()));
                } else {
                    body.push_str("\nNo human band here.\nF follows the next band.");
                }
                body
            } else {
                "Click any hex to inspect its\nclimate, resources, herds and people.\n\nThe planet wraps east-west.\nSeasons reverse across the equator.".into()
            };
        } else if matches!(kind, Hud::Chronicle) {
            **text = w
                .events
                .iter()
                .rev()
                .filter(|e| {
                    matches!(
                        e.kind,
                        EventKind::Hunt
                            | EventKind::Conflict
                            | EventKind::Exchange
                            | EventKind::Extinction
                    )
                })
                .take(4)
                .map(|e| {
                    let action = match e.kind {
                        EventKind::Hunt => format!("hunted {}", e.amount),
                        EventKind::Conflict => format!("conflict: {} lost", e.amount),
                        EventKind::Exchange => "peaceful exchange".into(),
                        _ => "band lost".into(),
                    };
                    format!("Day {} · Band {} · {}", e.day, e.tribe, action)
                })
                .collect::<Vec<_>>()
                .join("\n");
            if text.0.is_empty() {
                **text = "The story has only just begun.".into();
            }
        } else if matches!(kind, Hud::Status) {
            **text = if view.lod > 1 {
                format!(
                    "{}   /   Regional overview ({}x{} sampling)",
                    view.status, view.lod, view.lod
                )
            } else {
                view.status.clone()
            };
        }
    }
}
