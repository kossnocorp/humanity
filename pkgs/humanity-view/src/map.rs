use crate::{assets::Icons, *};
use bevy::{
    asset::RenderAssetUsages, camera::visibility::NoFrustumCulling, mesh::Indices,
    render::render_resource::PrimitiveTopology, window::PrimaryWindow,
};
use humanity_core::{Biome, Cell, PlanetConfig};

#[derive(Resource)]
pub struct MapMeshes {
    ground: Handle<Mesh>,
    detail: Handle<Mesh>,
}
#[derive(Component)]
pub struct Marker;

pub fn follow_band(
    game: Res<Game>,
    mut view: ResMut<ViewState>,
    mut camera: Single<(&mut Transform, &Projection), With<Camera2d>>,
) {
    let Some(id) = view.following else {
        return;
    };
    let Some(tribe) = game.world.tribes.iter().find(|t| t.id == id) else {
        view.following = None;
        view.status = format!("Band {id} no longer exists. F follows another band.");
        return;
    };
    let (transform, projection) = &mut *camera;
    let Projection::Orthographic(ortho) = projection else {
        return;
    };
    let p = position(tribe.cell, &game.world.config);
    // Center in the unobscured map area, accounting for the right field notes.
    let x = (p.x + 175. * ortho.scale).rem_euclid(game.world.config.width as f32 * HEX_WIDTH);
    let y = p.y + 19. * ortho.scale;
    if transform.translation.x != x
        || transform.translation.y != y
        || view.selected != Some(tribe.cell)
    {
        transform.translation.x = x;
        transform.translation.y = y;
        view.selected = Some(tribe.cell);
        view.dirty = true;
    }
    view.status = format!(
        "Following band {id} / {} people. F next band; pan to release.",
        tribe.headcount()
    );
}

pub fn position(id: CellId, c: &PlanetConfig) -> Vec2 {
    let y = id / c.width;
    Vec2::new(
        (id % c.width) as f32 * HEX_WIDTH + (y % 2) as f32 * HEX_WIDTH * 0.5,
        -(y as f32) * RADIUS * 1.5,
    )
}

pub fn cell_at(p: Vec2, c: &PlanetConfig) -> Option<CellId> {
    let row = (-p.y / (RADIUS * 1.5)).round() as i32;
    let mut nearest = None;
    let mut distance = RADIUS * RADIUS * 1.1;
    for y in row - 1..=row + 1 {
        if y < 0 || y >= c.height as i32 {
            continue;
        }
        let col = (p.x / HEX_WIDTH - (y % 2) as f32 * 0.5).round() as i32;
        for x in col - 1..=col + 1 {
            let center = Vec2::new(
                (x as f32 + (y % 2) as f32 * 0.5) * HEX_WIDTH,
                -y as f32 * RADIUS * 1.5,
            );
            let d = center.distance_squared(p);
            if d < distance {
                distance = d;
                nearest = Some(y as u32 * c.width + x.rem_euclid(c.width as i32) as u32);
            }
        }
    }
    nearest
}

pub fn setup(
    mut commands: Commands,
    game: Res<Game>,
    mut view: ResMut<ViewState>,
    mut meshes: ResMut<Assets<Mesh>>,
    mut materials: ResMut<Assets<ColorMaterial>>,
) {
    let focus = game
        .world
        .tribes
        .iter()
        .max_by_key(|t| t.headcount())
        .map_or(0, |t| t.cell);
    let p = position(focus, &game.world.config);
    view.selected = Some(focus);
    commands.spawn((
        Camera2d,
        Transform::from_xyz(p.x + 130., p.y, 1000.),
        Projection::Orthographic(OrthographicProjection {
            scale: 0.9,
            ..OrthographicProjection::default_2d()
        }),
    ));
    let ground = meshes.add(Geometry::default().mesh());
    let detail = meshes.add(Geometry::default().mesh());
    let material = materials.add(ColorMaterial::from(Color::WHITE));
    for (mesh, z) in [(&ground, 0.), (&detail, 1.)] {
        commands.spawn((
            Mesh2d(mesh.clone()),
            MeshMaterial2d(material.clone()),
            Transform::from_xyz(0., 0., z),
            NoFrustumCulling,
        ));
    }
    commands.insert_resource(MapMeshes { ground, detail });
}

#[derive(Default)]
struct Geometry {
    positions: Vec<[f32; 3]>,
    colors: Vec<[f32; 4]>,
    indices: Vec<u32>,
}
impl Geometry {
    fn polygon(&mut self, vertices: &[Vec2], color: [f32; 3]) {
        let start = self.positions.len() as u32;
        let linear = Color::srgb(color[0], color[1], color[2])
            .to_linear()
            .to_f32_array();
        for p in vertices {
            self.positions.push([p.x, p.y, 0.]);
            self.colors.push(linear);
        }
        for i in 1..vertices.len() as u32 - 1 {
            self.indices.extend([start, start + i, start + i + 1]);
        }
    }
    fn hex(&mut self, center: Vec2, radius: f32, color: [f32; 3]) {
        let points = std::array::from_fn::<_, 6, _>(|i| {
            let angle = std::f32::consts::TAU * i as f32 / 6. + std::f32::consts::FRAC_PI_2;
            center + Vec2::new(angle.cos(), angle.sin()) * radius
        });
        self.polygon(&points, color);
    }
    fn line(&mut self, a: Vec2, b: Vec2, width: f32, color: [f32; 3]) {
        let d = (b - a).normalize_or_zero().perp() * width / 2.;
        self.polygon(&[a - d, b - d, b + d, a + d], color);
    }
    fn mesh(self) -> Mesh {
        Mesh::new(
            PrimitiveTopology::TriangleList,
            RenderAssetUsages::default(),
        )
        .with_inserted_attribute(Mesh::ATTRIBUTE_POSITION, self.positions)
        .with_inserted_attribute(Mesh::ATTRIBUTE_COLOR, self.colors)
        .with_inserted_indices(Indices::U32(self.indices))
    }
}

fn color(c: &Cell, layer: Layer) -> [f32; 3] {
    match layer {
        Layer::Temperature => {
            let t = ((c.weather.temperature + 25.) / 65.).clamp(0., 1.);
            return [
                0.18 + t * 0.7,
                0.34 + (1. - (t - 0.5).abs() * 2.) * 0.25,
                0.8 - t * 0.6,
            ];
        }
        Layer::Food if !c.biome.water() => {
            let food = (c.resources.edible() / 15000.).clamp(0., 1.);
            return [0.32 + food * 0.16, 0.22 + food * 0.52, 0.15 + food * 0.15];
        }
        _ => {}
    }
    let base = match c.biome {
        Biome::Ocean => [0.075, 0.20, 0.29],
        Biome::Sea => [0.12, 0.32, 0.40],
        Biome::Lake => [0.20, 0.43, 0.51],
        Biome::Ice => [0.76, 0.83, 0.81],
        Biome::Tundra => [0.49, 0.55, 0.44],
        Biome::Taiga => [0.28, 0.44, 0.36],
        Biome::Grassland => [0.56, 0.65, 0.39],
        Biome::Forest => [0.34, 0.53, 0.34],
        Biome::Rainforest => [0.18, 0.42, 0.30],
        Biome::Savanna => [0.68, 0.64, 0.36],
        Biome::Desert => [0.78, 0.69, 0.46],
        Biome::Alpine => [0.52, 0.53, 0.48],
    };
    let variation = (c.elevation as f32 * 0.000025).clamp(-0.05, 0.06);
    let snow = c.weather.snow * if c.biome.water() { 0.3 } else { 0.78 };
    std::array::from_fn(|i| (base[i] + variation) * (1. - snow) + [0.88, 0.91, 0.86][i] * snow)
}

#[allow(clippy::too_many_arguments)]
pub fn redraw(
    mut commands: Commands,
    game: Res<Game>,
    mut view: ResMut<ViewState>,
    time: Res<Time>,
    window: Single<&Window, With<PrimaryWindow>>,
    camera: Single<(&Transform, &Projection), With<Camera2d>>,
    handles: Res<MapMeshes>,
    mut meshes: ResMut<Assets<Mesh>>,
    icons: Res<Icons>,
    markers: Query<Entity, With<Marker>>,
) {
    if !view.dirty
        && (game.world.day == view.last_day || time.elapsed_secs_f64() - view.last_draw < 0.15)
    {
        return;
    }
    let (transform, projection) = *camera;
    let Projection::Orthographic(ortho) = projection else {
        return;
    };
    let world = &game.world;
    let config = &world.config;
    let extent =
        Vec2::new(window.width(), window.height()) * ortho.scale * 0.5 + Vec2::splat(RADIUS * 2.);
    let center = transform.translation.truncate();
    let min = center - extent;
    let max = center + extent;
    let x0 = (min.x / HEX_WIDTH).floor() as i32 - 1;
    let x1 = (max.x / HEX_WIDTH).ceil() as i32 + 1;
    let y0 = ((-max.y / (RADIUS * 1.5)).floor() as i32 - 1).max(0);
    let y1 = ((-min.y / (RADIUS * 1.5)).ceil() as i32 + 1).min(config.height as i32 - 1);
    let count = ((x1 - x0 + 1).max(0) * (y1 - y0 + 1).max(0)) as usize;
    let stride = ((count as f32 / 9000.).sqrt().ceil() as usize).max(1);
    view.lod = stride;
    let mut ground = Geometry::default();
    let mut detail = Geometry::default();
    for entity in &markers {
        commands.entity(entity).despawn();
    }
    let mut decorations = 0;
    for y in (y0..=y1).step_by(stride) {
        for x in (x0..=x1).step_by(stride) {
            let id = y as u32 * config.width + x.rem_euclid(config.width as i32) as u32;
            let c = &world.cells[id as usize];
            let p = Vec2::new(
                (x as f32 + (y % 2) as f32 * 0.5) * HEX_WIDTH,
                -y as f32 * RADIUS * 1.5,
            );
            let rgb = color(c, view.layer);
            if stride == 1 {
                ground.hex(p, RADIUS, rgb.map(|v| v * 0.82));
                ground.hex(p, RADIUS * 0.956, rgb);
                if c.river > 0.
                    && let Some(to) = c.drainage
                {
                    let target = position(to, config);
                    let local = position(id, config);
                    let circumference = config.width as f32 * HEX_WIDTH;
                    let dx = (target.x - local.x + circumference / 2.).rem_euclid(circumference)
                        - circumference / 2.;
                    let next = p + Vec2::new(dx, target.y - local.y);
                    detail.line(
                        p,
                        next,
                        (0.6 + c.river.ln() * 0.3).min(3.),
                        [0.38, 0.72, 0.80],
                    );
                }
                if ortho.scale < 1.6
                    && decorations < 800
                    && (id.is_multiple_of(3) || c.elevation > 1700)
                    && matches!(view.layer, Layer::Biomes)
                {
                    let image = if c.elevation > 1700 {
                        Some(&icons.mountain)
                    } else if matches!(c.biome, Biome::Forest | Biome::Taiga | Biome::Rainforest) {
                        Some(&icons.forest)
                    } else {
                        None
                    };
                    if let Some(image) = image {
                        spawn_icon(&mut commands, image, p, 19., 2.);
                        decorations += 1;
                    }
                }
            } else {
                let a = Vec2::new(
                    x as f32 * HEX_WIDTH - HEX_WIDTH / 2.,
                    -y as f32 * RADIUS * 1.5 + RADIUS,
                );
                let b = a + Vec2::new(HEX_WIDTH * stride as f32, -RADIUS * 1.5 * stride as f32);
                ground.polygon(&[Vec2::new(a.x, b.y), b, Vec2::new(b.x, a.y), a], rgb);
            }
            if view.selected == Some(id) && stride == 1 {
                for i in 0..6 {
                    let a = std::f32::consts::TAU * i as f32 / 6. + std::f32::consts::FRAC_PI_2;
                    let b = a + std::f32::consts::TAU / 6.;
                    detail.line(
                        p + Vec2::new(a.cos(), a.sin()) * RADIUS * 0.97,
                        p + Vec2::new(b.cos(), b.sin()) * RADIUS * 0.97,
                        1.7 * ortho.scale.max(0.5),
                        [1., 0.89, 0.58],
                    );
                }
            }
        }
    }
    let circumference = config.width as f32 * HEX_WIDTH;
    let mut visible = 0;
    for (cell, image, offset, z) in world
        .herds
        .iter()
        .map(|h| (h.cell, &icons.herd, Vec2::new(5., -4.), 4.))
        .chain(
            world
                .tribes
                .iter()
                .map(|t| (t.cell, &icons.tribe, Vec2::new(-4., 5.), 5.)),
        )
    {
        if visible >= 2400 {
            break;
        }
        let p = position(cell, config);
        let x = center.x + (p.x - center.x + circumference / 2.).rem_euclid(circumference)
            - circumference / 2.;
        let p = Vec2::new(x, p.y);
        if p.x < min.x || p.x > max.x || p.y < min.y || p.y > max.y {
            continue;
        }
        spawn_icon(
            &mut commands,
            image,
            p + offset,
            (20. * ortho.scale.sqrt()).clamp(12., 60.),
            z,
        );
        visible += 1;
    }
    *meshes.get_mut(&handles.ground).expect("ground mesh") = ground.mesh();
    *meshes.get_mut(&handles.detail).expect("detail mesh") = detail.mesh();
    view.dirty = false;
    view.last_day = world.day;
    view.last_draw = time.elapsed_secs_f64();
}

fn spawn_icon(commands: &mut Commands, image: &Handle<Image>, p: Vec2, size: f32, z: f32) {
    commands.spawn((
        Marker,
        Sprite {
            image: image.clone(),
            custom_size: Some(Vec2::splat(size)),
            ..default()
        },
        Transform::from_xyz(p.x, p.y, z),
    ));
}

#[cfg(test)]
mod tests {
    use super::*;
    use bevy::ecs::system::RunSystemOnce;
    #[test]
    fn follow_tracks_a_stable_band_id_across_movement_zoom_and_extinction() {
        let world = humanity_world::generate(PlanetConfig {
            width: 24,
            height: 16,
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
        let id = world.tribes[0].id;
        let mut app = App::new();
        app.insert_resource(Game {
            world,
            simulation: Simulation::default(),
        });
        app.insert_resource(ViewState {
            following: Some(id),
            ..default()
        });
        let camera = app
            .world_mut()
            .spawn((
                Camera2d,
                Transform::default(),
                Projection::Orthographic(OrthographicProjection::default_2d()),
            ))
            .id();
        app.world_mut().run_system_once(follow_band).unwrap();
        let before = app.world().get::<Transform>(camera).unwrap().translation;
        let next = {
            let mut game = app.world_mut().resource_mut::<Game>();
            let cell = game.world.tribes[0].cell;
            let next = game.world.config.neighbors(cell).next().unwrap();
            game.world.tribes[0].cell = next;
            game.world.tribes.reverse();
            next
        };
        app.world_mut().run_system_once(follow_band).unwrap();
        assert_eq!(app.world().resource::<ViewState>().selected, Some(next));
        assert_ne!(
            app.world().get::<Transform>(camera).unwrap().translation,
            before
        );
        if let Some(mut projection) = app.world_mut().get_mut::<Projection>(camera)
            && let Projection::Orthographic(p) = &mut *projection
        {
            p.scale = 2.;
        }
        app.world_mut().run_system_once(follow_band).unwrap();
        let game = app.world().resource::<Game>();
        let p = position(next, &game.world.config);
        let actual = app.world().get::<Transform>(camera).unwrap().translation;
        assert!(
            (actual.x - (p.x + 350.).rem_euclid(game.world.config.width as f32 * HEX_WIDTH)).abs()
                < 0.001
        );
        assert!((actual.y - (p.y + 38.)).abs() < 0.001);
        app.world_mut()
            .resource_mut::<Game>()
            .world
            .tribes
            .retain(|t| t.id != id);
        app.world_mut().run_system_once(follow_band).unwrap();
        assert_eq!(app.world().resource::<ViewState>().following, None);
    }
    #[test]
    fn hex_picking_round_trips_and_wraps() {
        let c = PlanetConfig {
            width: 24,
            height: 16,
            cell_km: 20.,
            temperature: 28.,
            season_days: 30,
            axial_tilt: 23.4,
            ocean_fraction: 0.6,
            rainfall: 1.,
            population: 100,
            seed: 1,
        };
        for id in 0..c.len() as u32 {
            let p = position(id, &c);
            assert_eq!(cell_at(p, &c), Some(id));
            assert_eq!(
                cell_at(p + Vec2::X * c.width as f32 * HEX_WIDTH, &c),
                Some(id)
            );
        }
        assert_eq!(cell_at(Vec2::new(0., 100.), &c), None);
    }
}
