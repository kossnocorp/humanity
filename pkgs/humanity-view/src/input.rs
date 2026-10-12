use crate::*;
use bevy::{
    input::{
        ButtonState,
        keyboard::{Key, KeyboardInput},
        mouse::{MouseScrollUnit, MouseWheel},
    },
    window::PrimaryWindow,
};

#[allow(clippy::too_many_arguments)]
pub fn controls(
    keys: Res<ButtonInput<KeyCode>>,
    mouse: Res<ButtonInput<MouseButton>>,
    mut wheel: MessageReader<MouseWheel>,
    time: Res<Time>,
    window: Single<&Window, With<PrimaryWindow>>,
    mut game: ResMut<Game>,
    mut view: ResMut<ViewState>,
    mut keyboard: MessageReader<KeyboardInput>,
) {
    let question = keyboard
        .read()
        .filter(|e| {
            e.state == ButtonState::Pressed
                && !e.repeat
                && matches!(&e.logical_key, Key::Character(c) if c.as_str() == "?")
        })
        .count()
        > 0;
    if question
        || (keys.just_pressed(KeyCode::Slash)
            && keys.any_pressed([KeyCode::ShiftLeft, KeyCode::ShiftRight]))
    {
        view.insights_open = !view.insights_open;
        view.drag_start = None;
    }
    if keys.just_pressed(KeyCode::Escape) {
        view.insights_open = false;
        if let Some(id) = view.following
            && let Some(i) = game.world.tribes.iter().position(|t| t.id == id)
        {
            view.focus_index = i;
        }
        view.following = None;
        view.selected = None;
        view.dirty = true;
        view.status = "Focus cleared / F follows the next band.".into();
    }
    if keys.just_pressed(KeyCode::Space) {
        view.paused = !view.paused;
        view.accumulator = 0.;
    }
    if keys.any_just_pressed([KeyCode::Equal, KeyCode::NumpadAdd]) {
        view.speed = (view.speed * 2.).min(256.);
    }
    if keys.any_just_pressed([KeyCode::Minus, KeyCode::NumpadSubtract]) {
        view.speed = (view.speed / 2.).max(0.25);
    }
    if keys.just_pressed(KeyCode::Tab) {
        view.layer = view.layer.next();
        view.dirty = true;
    }
    if keys.just_pressed(KeyCode::KeyS) {
        view.status = match humanity_save::save(&game.world, &view.save_path) {
            Ok(()) => format!(
                "Saved day {} / {}",
                game.world.day,
                view.save_path.display()
            ),
            Err(e) => format!("Save failed: {e:#}"),
        };
    }
    if keys.just_pressed(KeyCode::KeyL) {
        match humanity_save::load(&view.save_path) {
            Ok(world) => {
                game.world = world;
                game.simulation = Simulation::default();
                view.snapshot_revision = view.snapshot_revision.wrapping_add(1);
                view.paused = true;
                view.accumulator = 0.;
                view.dirty = true;
                view.selected = None;
                view.following = None;
                view.status = format!("Loaded day {} / paused", game.world.day);
            }
            Err(e) => view.status = format!("Load failed: {e:#}"),
        }
    }
    if view.insights_open {
        wheel.clear();
        return;
    }
    let size = Vec2::new(window.width(), window.height());
    if keys.just_pressed(KeyCode::Home) {
        view.following = None;
        view.distance = PLANET_RADIUS * 3.2;
        view.dirty = true;
        view.status = "3D globe / drag or arrows to rotate across either pole.".into();
    }
    if keys.just_pressed(KeyCode::KeyF) && !game.world.tribes.is_empty() {
        if let Some(id) = view.following
            && let Some(i) = game.world.tribes.iter().position(|t| t.id == id)
        {
            view.focus_index = i;
        }
        view.focus_index = (view.focus_index + 1) % game.world.tribes.len();
        let tribe = &game.world.tribes[view.focus_index];
        view.following = Some(tribe.id);
        view.selected = Some(tribe.cell);
        view.distance = PLANET_RADIUS
            + (map::cell_spacing(&game.world) * 12.)
                .clamp(PLANET_RADIUS * 0.025, PLANET_RADIUS * 1.4);
        view.dirty = true;
    }
    let direction = Vec2::new(
        f32::from(keys.pressed(KeyCode::ArrowRight)) - f32::from(keys.pressed(KeyCode::ArrowLeft)),
        f32::from(keys.pressed(KeyCode::ArrowUp)) - f32::from(keys.pressed(KeyCode::ArrowDown)),
    );
    if direction != Vec2::ZERO {
        view.following = None;
        let speed =
            ((view.distance - PLANET_RADIUS) / PLANET_RADIUS).clamp(0.025, 1.5) * time.delta_secs();
        view.orbit = (view.orbit
            * Quat::from_rotation_y(direction.x * speed)
            * Quat::from_rotation_x(-direction.y * speed))
        .normalize();
        view.dirty = true;
    }
    let cursor = window.cursor_position();
    let on_map = cursor.is_some_and(|p| p.y > 92. && p.y < size.y - 54. && p.x < size.x - 350.);
    for e in wheel.read() {
        if !on_map {
            continue;
        }
        let amount = match e.unit {
            MouseScrollUnit::Line => e.y,
            MouseScrollUnit::Pixel => e.y / 60.,
        };
        let before = cursor.and_then(|p| map::screen_to_sphere(p, size, view.orbit, view.distance));
        view.distance = PLANET_RADIUS
            + ((view.distance - PLANET_RADIUS) * (-amount * 0.16).exp())
                .clamp(PLANET_RADIUS * 0.015, PLANET_RADIUS * 6.);
        if view.following.is_none()
            && let (Some(a), Some(b)) = (
                before,
                cursor.and_then(|p| map::screen_to_sphere(p, size, view.orbit, view.distance)),
            )
        {
            view.orbit = (Quat::from_rotation_arc(b, a) * view.orbit).normalize();
        }
        view.dirty = true;
    }
    if mouse.just_pressed(MouseButton::Left) && on_map {
        view.drag_start = cursor;
        view.last_cursor = cursor;
        view.dragged = false;
    }
    if mouse.pressed(MouseButton::Left)
        && let (Some(start), Some(last), Some(p)) = (view.drag_start, view.last_cursor, cursor)
    {
        if start.distance(p) > 4. {
            view.dragged = true;
        }
        if view.dragged {
            view.following = None;
            let delta = p - last;
            let sensitivity = 2. * (CAMERA_FOV * 0.5).tan() * (view.distance - PLANET_RADIUS)
                / (PLANET_RADIUS * size.y.max(1.));
            view.orbit = (view.orbit
                * Quat::from_rotation_y(-delta.x * sensitivity)
                * Quat::from_rotation_x(-delta.y * sensitivity))
            .normalize();
            view.dirty |= delta != Vec2::ZERO;
        }
        view.last_cursor = Some(p);
    }
    if mouse.just_released(MouseButton::Left) {
        if view.drag_start.is_some()
            && !view.dragged
            && on_map
            && let Some(p) = cursor
        {
            view.selected = map::screen_to_sphere(p, size, view.orbit, view.distance)
                .map(|p| game.world.grid.nearest(p));
            view.following = None;
            view.dirty = true;
        }
        view.drag_start = None;
        view.last_cursor = None;
    }
    view.dirty |= size != view.last_size;
    view.last_size = size;
}
