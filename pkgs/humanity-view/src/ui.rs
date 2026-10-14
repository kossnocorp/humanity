use crate::*;
use bevy::{
    input::mouse::{MouseScrollUnit, MouseWheel},
    window::PrimaryWindow,
};
use humanity_core::{EventKind, Skill, Species};

#[derive(Component)]
pub struct FieldNotes;

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
                        Text::new("CAMPS, CULTURES & SEASONS"),
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
    commands.spawn((FieldNotes, ScrollPosition::default(), Node { position_type: PositionType::Absolute, right: px(20), top: px(110), bottom: px(72),
        width: px(310), padding: UiRect::all(px(20)), flex_direction: FlexDirection::Column,
        row_gap: px(15), overflow: Overflow::scroll_y(), ..default() },
        BackgroundColor(Color::srgba(0.045, 0.075, 0.09, 0.96))))
        .with_children(|p| {
            p.spawn((Text::new("FIELD NOTES / scroll for culture"), TextFont { font_size: 15., ..default() }, TextColor(GOLD)));
            p.spawn((Hud::Notes, Text::default(), TextFont { font_size: 14., ..default() }, TextColor(INK)));
            p.spawn((Text::new("RECENT ENCOUNTERS"), TextFont { font_size: 12., ..default() }, TextColor(GOLD)));
            p.spawn((Hud::Chronicle, Text::default(), TextFont { font_size: 12., ..default() }, TextColor(MUTED)));
            p.spawn((Text::new("Cream camps: human bands\nCyan houses: fishing locations\nViolet tents: remembered camps\nOchre animals: migrating herds\nBlue threads: rivers  /  Pale ground: snow"),
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
            p.spawn((Text::new("SPACE pause   - / + speed   ARROWS rotate   WHEEL zoom   S/L save/load   TAB layer   F follow   B browser   ALT+ARROWS menu   ENTER open   ? population"),
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
                    "CELL {} / {} / {:?}\nLat {:+.1} / Lon {:+.1}\n{} m elevation / ~{:.0} km spacing\nPlanet radius {:.0} km / river {:.0}\n\nWEATHER & LIVING LAND\n{:.1} C   /   rain {:.1} mm/day\nSnow {:.0}%   /   soil moisture {:.0}%\nPlants {:>7.0}   Fruit {:>7.0}\nGame {:>8.0}   Graze {:>6.0}\nFood: person-days; graze: animal-days\n{} herd animals here\n",
                    id,
                    if w.grid.degree(id) == 5 {
                        "pentagon"
                    } else {
                        "hexagon"
                    },
                    c.biome,
                    w.grid.latitude(id) * 90.,
                    w.grid.longitude(id).to_degrees(),
                    c.elevation,
                    w.config.cell_km,
                    w.config.radius_km(),
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
                let r = &c.resources;
                if let Some(human_id) = view.selected_human
                    && let Some((t, h)) = w.tribes.iter().find_map(|t| {
                        t.people
                            .members
                            .iter()
                            .find(|h| h.id == human_id)
                            .map(|h| (t, h))
                    })
                {
                    let mut personal = format!(
                        "HUMAN {} / {:?} / age {}\nBand {} / {:?} / health {:.0}%\n\nPERSONAL SKILLS (level / practiced days)\n",
                        h.id,
                        h.sex,
                        h.age(w.day, w.config.year_days()),
                        t.id,
                        h.role,
                        h.health * 100.
                    );
                    for skill in humanity_core::Skill::ALL {
                        let i = skill as usize;
                        personal.push_str(&format!(
                            "{}: {:.1} / {:.1}d\n  taught +{:.1}, stories +{:.1}\n",
                            skill.label(),
                            h.skills.level(skill) * 100.,
                            h.skills.experience[i],
                            h.skills.taught[i] * 100.,
                            h.skills.stories[i] * 100.
                        ));
                    }
                    personal.push_str("\nRECENT PERSONAL HISTORY\n");
                    for e in h.history.iter().rev().take(12) {
                        personal.push_str(&format!("D{}: {:?}\n", e.day, e.kind));
                    }
                    personal.push('\n');
                    personal.push_str(&body);
                    body = personal;
                }
                body.push_str(&format!(
                    "Near-camp depletion {:.2} / search yield {:.0}%\n",
                    r.foraging_pressure,
                    100. / (1. + r.foraging_pressure)
                ));
                body.push_str(&format!("\nAQUATIC FOOD (person-days)\nFreshwater fish {:.0}\nCoastal / pelagic fish {:.0}\nShellfish {:.0}\n\nCRAFT MATERIALS\nWood {:.0} / Fiber {:.0}\nToolstone availability {:.0}%\n",
                    r.aquatic.freshwater_fish, r.aquatic.coastal_fish, r.aquatic.shellfish, r.wood, r.fiber, r.toolstone*100.));
                for h in w.herds.iter().filter(|h| h.cell == id) {
                    body.push_str(&format!(
                        "{:?}: {} / condition {:.0}%\n",
                        h.species,
                        h.animals,
                        h.condition * 100.
                    ));
                }
                for s in w.sites.iter().filter(|s| s.cell == id) {
                    let o = &s.occupation;
                    body.push_str(&format!("\n\nLOCATION / {}\n{} / culture {}\n{}\nKnown seasons: {}\nCondition {:.0}% / maintained day {}\nFounded {} / last visit {}\n{} associated people present\nCurrent stay {} days / longest {}\n{} occupied days in {} stays\n",
                        s.name, s.kind.label(), s.culture, s.pattern(w.config.season_days),
                        s.seasons_label(), s.condition*100., s.last_maintained.map_or("never".into(), |day| day.to_string()),
                        s.founded, s.last_visited, o.present_people, o.continuous_days, o.longest_stay,
                        o.occupied_days, o.episodes));
                    body.push_str("Recent seasons: occupied days / person-days\n");
                    for season in &o.seasons {
                        body.push_str(&format!(
                            "Year {} {}: {} / {}\n",
                            season.period / 4 + 1,
                            ["spring", "summer", "autumn", "winter"][season.season as usize],
                            season.occupied_days,
                            season.person_days
                        ));
                    }
                }
                if let Some(t) = people
                    .iter()
                    .find(|t| Some(t.id) == view.following)
                    .or(people.first())
                {
                    let languages = t
                        .languages
                        .iter()
                        .map(|l| format!("{}: {:.0}%", l.family, l.share * 100.))
                        .collect::<Vec<_>>()
                        .join("  ");
                    let children = t.people.female[0] + t.people.male[0];
                    let elders = t.people.female[2] + t.people.male[2];
                    let culture = &w.cultures[t.culture as usize];
                    body.push_str(&format!("\n\nCULTURE {} / {}\n{}\nParents: {:?}\nIsolation {} days\nMixing contacts {} / {} days\n",
                        culture.id, culture.name, culture.variant.label(), culture.parents,
                        t.isolation_days, t.contact_days, humanity_core::SOCIAL.mixing_contact_days));
                    body.push_str(&format!("\n\nCAMP CUSTOM / {}\nCurrent split threshold: {} people\nCrowding pressure: {} / {} days\n",
                        t.camp_tradition.label(), w.camp_limit(t), t.camp_pressure_days, t.camp_tradition.pressure_days()));
                    if let Some(parent) = t.parent_band {
                        body.push_str(&format!("Offshoot of band {parent}\n"));
                    }
                    body.push_str(&format!("\nBAND {}   /   {} people\nHealth {:.0}%  /  food {:.1} days\nFemale {}  /  Male {}\nYoung {}  /  Adult {}  /  Elder {}\nHunters {}  /  Gatherers {} / Fishers {}\nHealers {}  /  Guides {}\nLanguage families\n{}\n{} band(s) share this cell",
                        t.id, t.headcount(), t.health * 100., t.food / t.headcount().max(1) as f32,
                        t.people.female.iter().sum::<u32>(), t.people.male.iter().sum::<u32>(), children, t.people.adults(), elders,
                         t.specialists.hunters, t.specialists.gatherers, t.specialists.fishers, t.specialists.healers, t.specialists.guides, languages, people.len()));
                    body.push_str(&format!("\n\nLIVELIHOOD / KNOWLEDGE\nFishing workers today {:.0}%\nBoat places {:.0} / {} people\nCarried wood {:.0} / fiber {:.0}\nStone {:.0} / hides {:.0}\n\nMean skill: child / adult / elder\n",
                        t.fishing_effort*100., t.equipment.boat_places,t.headcount(),t.equipment.wood,t.equipment.fiber,t.equipment.stone,t.equipment.hides));
                    for skill in Skill::ALL {
                        let k = t.knowledge.practitioners[skill as usize];
                        body.push_str(&format!(
                            "{}\n  {:.0}% / {:.0}% / {:.0}%\n",
                            skill.label(),
                            k[0] * 100.,
                            k[1] * 100.,
                            k[2] * 100.
                        ));
                    }
                    body.push_str("\nHUNTING FAMILIARITY\n");
                    for species in Species::ALL {
                        body.push_str(&format!(
                            "{:?}: {:.0}%\n",
                            species,
                            t.knowledge.prey[species.index()] * 100.
                        ));
                    }
                } else {
                    body.push_str("\nNo human band here.\nF follows the next band.");
                }
                body
            } else {
                "Click a cell on the globe to inspect\nclimate, resources, herds and people.\n\nA closed sphere: 12 pentagons,\nall remaining cells hexagons.\nDrag freely across either pole.\nSeasons reverse across the equator.".into()
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
                            | EventKind::KnowledgeExchange
                            | EventKind::Innovation
                            | EventKind::WaterCrossing
                            | EventKind::Fishing
                            | EventKind::BandFission
                            | EventKind::CultureDivergence
                            | EventKind::CultureMixing
                            | EventKind::SiteFounded
                            | EventKind::SiteAbandoned
                    )
                })
                .take(4)
                .map(|e| {
                    let action = match e.kind {
                        EventKind::Hunt => format!("hunted {}", e.amount),
                        EventKind::Conflict => format!("conflict: {} lost", e.amount),
                        EventKind::Exchange => "peaceful exchange".into(),
                        EventKind::KnowledgeExchange => format!(
                            "shared {}",
                            Skill::ALL
                                .get(e.amount as usize)
                                .map_or("knowledge", |s| s.label())
                        ),
                        EventKind::Innovation => format!(
                            "experiment: {}",
                            Skill::ALL
                                .get(e.amount as usize)
                                .map_or("knowledge", |s| s.label())
                        ),
                        EventKind::WaterCrossing => "crossed water by boat".into(),
                        EventKind::Fishing => format!("fished {} food-days", e.amount),
                        EventKind::BandFission => {
                            format!("{} people formed band {}", e.amount, e.other.unwrap_or(0))
                        }
                        EventKind::CultureDivergence => {
                            format!("isolation: culture {} formed", e.amount)
                        }
                        EventKind::CultureMixing => format!("joined mixed culture {}", e.amount),
                        EventKind::SiteFounded => "established a remembered location".into(),
                        EventKind::SiteAbandoned => "camp faded / abandoned".into(),
                        _ => "band lost".into(),
                    };
                    format!("Day {} / Band {} / {}", e.day, e.tribe, action)
                })
                .collect::<Vec<_>>()
                .join("\n");
            if text.0.is_empty() {
                **text = "The story has only just begun.".into();
            }
        } else if matches!(kind, Hud::Status) {
            **text = if view.lod > 1 {
                format!(
                    "{}   /   Orbital overview: zoom in for individual cells",
                    view.status
                )
            } else {
                view.status.clone()
            };
        }
    }
}

pub fn scroll(
    mut wheel: MessageReader<MouseWheel>,
    window: Single<&Window, With<PrimaryWindow>>,
    view: Res<ViewState>,
    mut panels: Query<&mut ScrollPosition, With<FieldNotes>>,
) {
    for e in wheel.read() {
        if view.insights_open
            || !window.cursor_position().is_some_and(|p| {
                p.x > window.width() - 350. && p.y > 100. && p.y < window.height() - 72.
            })
        {
            continue;
        }
        let delta = match e.unit {
            MouseScrollUnit::Line => e.y * 40.,
            MouseScrollUnit::Pixel => e.y,
        };
        for mut pos in &mut panels {
            pos.0.y = (pos.0.y - delta).max(0.);
        }
    }
}
