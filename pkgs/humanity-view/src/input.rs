use crate::*;
use bevy::{
    input::mouse::{MouseScrollUnit, MouseWheel},
    window::PrimaryWindow,
};

#[allow(clippy::too_many_arguments)]
pub fn controls(
    keys: Res<ButtonInput<KeyCode>>,
    mouse: Res<ButtonInput<MouseButton>>,
    mut wheel: MessageReader<MouseWheel>,
    time: Res<Time>,
    window: Single<&Window, With<PrimaryWindow>>,
    mut camera: Single<(&mut Transform, &mut Projection), With<Camera2d>>,
    mut game: ResMut<Game>,
    mut view: ResMut<ViewState>,
) {
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
                "Saved day {} · {}",
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
                view.paused = true;
                view.accumulator = 0.;
                view.dirty = true;
                view.selected = None;
                view.status = format!("Loaded day {} · paused", game.world.day);
            }
            Err(e) => view.status = format!("Load failed: {e:#}"),
        }
    }
    let (transform, projection) = &mut *camera;
    let Projection::Orthographic(ortho) = &mut **projection else {
        return;
    };
    let size = Vec2::new(window.width(), window.height());
    let map_width = game.world.config.width as f32 * HEX_WIDTH;
    let map_height = game.world.config.height as f32 * RADIUS * 1.5;
    let fit = (map_width / (size.x - 360.).max(300.)).max(map_height / (size.y - 150.).max(200.));
    if keys.just_pressed(KeyCode::Home) {
        ortho.scale = fit * 1.08;
        transform.translation.x = map_width / 2. + 160. * ortho.scale;
        transform.translation.y = -map_height / 2.;
        view.dirty = true;
    }
    if keys.just_pressed(KeyCode::KeyF) && !game.world.tribes.is_empty() {
        view.focus_index = (view.focus_index + 1) % game.world.tribes.len();
        let cell = game.world.tribes[view.focus_index].cell;
        let p = map::position(cell, &game.world.config);
        transform.translation.x = p.x + 140.;
        transform.translation.y = p.y;
        ortho.scale = 0.85;
        view.selected = Some(cell);
        view.dirty = true;
    }
    let direction = Vec2::new(
        f32::from(keys.pressed(KeyCode::ArrowRight)) - f32::from(keys.pressed(KeyCode::ArrowLeft)),
        f32::from(keys.pressed(KeyCode::ArrowUp)) - f32::from(keys.pressed(KeyCode::ArrowDown)),
    );
    if direction != Vec2::ZERO {
        transform.translation +=
            (direction.normalize() * 620. * ortho.scale * time.delta_secs()).extend(0.);
        view.dirty = true;
    }
    let cursor = window.cursor_position();
    let on_map = cursor.is_some_and(|p| p.y > 92. && p.y < size.y - 54. && p.x < size.x - 350.);
    for event in wheel.read() {
        if !on_map {
            continue;
        }
        let amount = match event.unit {
            MouseScrollUnit::Line => event.y,
            MouseScrollUnit::Pixel => event.y / 60.,
        };
        let previous = ortho.scale;
        ortho.scale = (ortho.scale * (-amount * 0.16).exp()).clamp(0.18, fit * 1.3);
        if let Some(p) = cursor {
            let offset = Vec2::new(p.x - size.x / 2., size.y / 2. - p.y);
            transform.translation += (offset * (previous - ortho.scale)).extend(0.);
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
            let delta = p - last;
            transform.translation += Vec3::new(-delta.x, delta.y, 0.) * ortho.scale;
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
            let world_pos = transform.translation.truncate()
                + Vec2::new(p.x - size.x / 2., size.y / 2. - p.y) * ortho.scale;
            view.selected = map::cell_at(world_pos, &game.world.config);
            view.dirty = true;
        }
        view.drag_start = None;
        view.last_cursor = None;
    }
    let old = transform.translation;
    transform.translation.x = transform.translation.x.rem_euclid(map_width);
    transform.translation.y = transform.translation.y.clamp(-map_height, 0.);
    view.dirty |= old != transform.translation || size != view.last_size;
    view.last_size = size;
}
