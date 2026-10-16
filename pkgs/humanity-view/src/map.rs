use crate::{assets::Icons, *};
use bevy::{
    asset::RenderAssetUsages, camera::visibility::NoFrustumCulling,
    core_pipeline::tonemapping::Tonemapping, mesh::Indices,
    render::render_resource::PrimitiveTopology, window::PrimaryWindow,
};
use humanity_core::{Biome, Cell};
use humanity_grid::Grid;
use std::collections::{HashSet, VecDeque};

#[derive(Component)]
pub struct Marker;
#[derive(Component)]
pub struct GlobeLight;
#[derive(Resource)]
pub struct MapMeshes {
    base: Handle<Mesh>,
    detail: Handle<Mesh>,
    rivers: Handle<Mesh>,
    quad: Handle<Mesh>,
    tribe: Handle<StandardMaterial>,
    herd: Handle<StandardMaterial>,
    small_fauna: Handle<StandardMaterial>,
    forest: Handle<StandardMaterial>,
    mountain: Handle<StandardMaterial>,
    fish: Handle<StandardMaterial>,
    fishing_site: Handle<StandardMaterial>,
    seasonal_site: Handle<StandardMaterial>,
    coarse: Grid,
    coarse_cells: Vec<u32>,
    revision: u64,
    base_day: u64,
    base_layer: &'static str,
}

pub fn cell_spacing(world: &World) -> f32 {
    PLANET_RADIUS * world.config.cell_km / world.config.radius_km()
}

/// The camera orbits with a quaternion, so neither pole is singular or bounded.
pub fn orbit_camera(
    game: Res<Game>,
    mut view: ResMut<ViewState>,
    mut camera: Single<&mut Transform, (With<Camera3d>, Without<GlobeLight>)>,
    mut lights: Query<&mut Transform, (With<GlobeLight>, Without<Camera3d>)>,
) {
    if let Some(id) = view.following {
        if let Some(tribe) = game.world.tribes.iter().find(|t| t.id == id) {
            let direction = game.world.grid.direction(tribe.cell);
            let previous = view.orbit * Vec3::Z;
            if previous.distance_squared(direction) > 1e-12 || view.selected != Some(tribe.cell) {
                view.orbit =
                    (Quat::from_rotation_arc(previous, direction) * view.orbit).normalize();
                view.selected = Some(tribe.cell);
                view.dirty = true;
            }
            view.status = format!(
                "Following band {id} / {} people. F next band; Esc unfollow.",
                tribe.headcount()
            );
        } else {
            view.following = None;
            view.status = format!("Band {id} no longer exists. F follows another band.");
        }
    }
    camera.translation = view.orbit * Vec3::Z * view.distance;
    camera.rotation = view.orbit;
    // Readable observation lighting, not an extra simulation day/night mechanic.
    for mut light in &mut lights {
        *light =
            Transform::from_translation(view.orbit * Vec3::new(-0.7, 0.9, 1.4) * PLANET_RADIUS)
                .looking_at(Vec3::ZERO, view.orbit * Vec3::Y);
    }
}

pub fn screen_to_sphere(cursor: Vec2, size: Vec2, orbit: Quat, distance: f32) -> Option<Vec3> {
    if size.x <= 0. || size.y <= 0. {
        return None;
    }
    let ndc = Vec2::new(cursor.x / size.x * 2. - 1., 1. - cursor.y / size.y * 2.);
    let tan = (CAMERA_FOV * 0.5).tan();
    let ray = orbit * Vec3::new(ndc.x * size.x / size.y * tan, ndc.y * tan, -1.).normalize();
    ray_sphere(orbit * Vec3::Z * distance, ray)
}

fn ray_sphere(origin: Vec3, direction: Vec3) -> Option<Vec3> {
    // Double precision keeps very close surface picks stable on large grids.
    let o = origin.as_dvec3();
    let d = direction.as_dvec3();
    let b = o.dot(d);
    let c = o.length_squared() - f64::from(PLANET_RADIUS).powi(2);
    let a = d.length_squared();
    let discriminant = b * b - a * c;
    if discriminant < 0. {
        return None;
    }
    let distance = (-b - discriminant.sqrt()) / a;
    if distance < 0. {
        return None;
    }
    Some((o + d * distance).normalize().as_vec3())
}

pub fn setup(
    mut commands: Commands,
    game: Res<Game>,
    mut view: ResMut<ViewState>,
    icons: Res<Icons>,
    mut meshes: ResMut<Assets<Mesh>>,
    mut materials: ResMut<Assets<StandardMaterial>>,
) {
    let world = &game.world;
    let focus = view
        .following
        .and_then(|id| world.tribes.iter().find(|t| t.id == id))
        .or_else(|| world.tribes.iter().max_by_key(|t| t.headcount()))
        .map_or(0, |t| t.cell);
    if view.following.is_some() {
        view.distance = PLANET_RADIUS
            + (cell_spacing(world) * 12.).clamp(PLANET_RADIUS * 0.025, PLANET_RADIUS * 1.4);
    }
    view.selected = Some(focus);
    view.orbit = Quat::from_rotation_arc(Vec3::Z, world.grid.direction(focus));
    view.status =
        "3D globe / drag or arrows to rotate; F follows a band; click a cell to inspect.".into();
    commands.spawn((
        Camera3d::default(),
        Tonemapping::None,
        Projection::Perspective(PerspectiveProjection {
            fov: CAMERA_FOV,
            near: 0.01,
            far: PLANET_RADIUS * 30.,
            ..default()
        }),
        Transform {
            translation: view.orbit * Vec3::Z * view.distance,
            rotation: view.orbit,
            ..default()
        },
    ));
    commands.insert_resource(GlobalAmbientLight {
        color: Color::WHITE,
        brightness: 550.,
        ..default()
    });
    commands.spawn((
        GlobeLight,
        DirectionalLight {
            illuminance: 6500.,
            shadows_enabled: false,
            ..default()
        },
        Transform::default(),
    ));
    let base = meshes.add(Geometry::default().mesh());
    let detail = meshes.add(Geometry::default().mesh());
    let rivers = meshes.add(Geometry::default().mesh());
    let ground_material = materials.add(StandardMaterial {
        base_color: Color::WHITE,
        perceptual_roughness: 1.,
        reflectance: 0.,
        ..default()
    });
    let line_material = materials.add(StandardMaterial {
        base_color: Color::WHITE,
        unlit: true,
        ..default()
    });
    for (mesh, material) in [
        (&base, &ground_material),
        (&detail, &ground_material),
        (&rivers, &line_material),
    ] {
        commands.spawn((
            Mesh3d(mesh.clone()),
            MeshMaterial3d(material.clone()),
            Transform::default(),
            NoFrustumCulling,
        ));
    }
    let mut icon_material = |image: &Handle<Image>| {
        materials.add(StandardMaterial {
            base_color_texture: Some(image.clone()),
            unlit: true,
            alpha_mode: AlphaMode::Blend,
            cull_mode: None,
            ..default()
        })
    };
    let coarse = Grid::new(world.config.resolution.min(28)).expect("validated resolution");
    let coarse_cells = (0..coarse.len() as u32)
        .map(|i| world.grid.nearest(coarse.direction(i)))
        .collect();
    commands.insert_resource(MapMeshes {
        base,
        detail,
        rivers,
        quad: meshes.add(Rectangle::new(1., 1.)),
        tribe: icon_material(&icons.tribe),
        herd: icon_material(&icons.herd),
        small_fauna: icon_material(&icons.small_fauna),
        forest: icon_material(&icons.forest),
        mountain: icon_material(&icons.mountain),
        fish: icon_material(&icons.fish),
        fishing_site: icon_material(&icons.fishing_site),
        seasonal_site: icon_material(&icons.seasonal_site),
        coarse,
        coarse_cells,
        revision: view.snapshot_revision,
        base_day: u64::MAX,
        base_layer: "",
    });
}

#[derive(Default)]
struct Geometry {
    positions: Vec<[f32; 3]>,
    normals: Vec<[f32; 3]>,
    colors: Vec<[f32; 4]>,
    indices: Vec<u32>,
}
impl Geometry {
    fn vertex(&mut self, p: Vec3, color: [f32; 3]) {
        self.positions.push(p.to_array());
        self.normals.push(p.normalize().to_array());
        self.colors.push(
            Color::srgb(color[0], color[1], color[2])
                .to_linear()
                .to_f32_array(),
        );
    }
    fn cell(&mut self, grid: &Grid, id: u32, radius: f32, color: [f32; 3], inset: f32) {
        let center = grid.direction(id);
        let start = self.positions.len() as u32;
        self.vertex(center * radius, color);
        for corner in grid.corners(id) {
            self.vertex(center.lerp(corner, inset).normalize() * radius, color);
        }
        let count = grid.degree(id) as u32;
        for i in 0..count {
            self.indices
                .extend([start, start + 1 + i, start + 1 + (i + 1) % count]);
        }
    }
    fn line(&mut self, a: Vec3, b: Vec3, radius: f32, width: f32, color: [f32; 3]) {
        for step in 0..3 {
            let p = a.lerp(b, step as f32 / 3.).normalize() * radius;
            let q = a.lerp(b, (step + 1) as f32 / 3.).normalize() * radius;
            let side = (p + q).normalize().cross((q - p).normalize_or_zero()) * width * 0.5;
            let start = self.positions.len() as u32;
            for point in [p - side, q - side, q + side, p + side] {
                self.vertex(point, color);
            }
            self.indices
                .extend([start, start + 1, start + 2, start, start + 2, start + 3]);
        }
    }
    fn mesh(self) -> Mesh {
        Mesh::new(
            PrimitiveTopology::TriangleList,
            RenderAssetUsages::default(),
        )
        .with_inserted_attribute(Mesh::ATTRIBUTE_POSITION, self.positions)
        .with_inserted_attribute(Mesh::ATTRIBUTE_NORMAL, self.normals)
        .with_inserted_attribute(Mesh::ATTRIBUTE_COLOR, self.colors)
        .with_inserted_indices(Indices::U32(self.indices))
    }
}

fn color(c: &Cell, layer: Layer) -> [f32; 3] {
    match layer {
        Layer::Aquatic => {
            let f = (c.resources.aquatic.total() / 12000.).sqrt().clamp(0., 1.);
            return [0.06 + f * 0.2, 0.12 + f * 0.65, 0.19 + f * 0.65];
        }
        Layer::Materials => {
            return [
                0.1 + c.resources.toolstone * 0.65,
                0.1 + (c.resources.wood / 2500.).clamp(0., 1.) * 0.65,
                0.1 + (c.resources.fiber / 1000.).clamp(0., 1.) * 0.65,
            ];
        }
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
        Biome::Steppe => [0.65, 0.61, 0.43],
        Biome::Scrubland => [0.58, 0.55, 0.32],
        Biome::Marsh => [0.35, 0.55, 0.49],
        Biome::Swamp => [0.19, 0.36, 0.33],
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

/// A spherical-cap traversal touches only the visible patch at close zoom.
/// A bounded coarse globe remains behind it, so even multi-million-cell maps
/// never require one entity or one permanently uploaded polygon per cell.
fn visible_cells(grid: &Grid, axis: Vec3, threshold: f32, limit: usize) -> Vec<u32> {
    let start = grid.nearest(axis);
    let mut seen = HashSet::new();
    let mut queue = VecDeque::from([start]);
    let mut result = Vec::new();
    seen.insert(start);
    while let Some(id) = queue.pop_front() {
        result.push(id);
        if result.len() == limit {
            break;
        }
        for next in grid.neighbors(id) {
            if seen.insert(next) && grid.direction(next).dot(axis) >= threshold {
                queue.push_back(next);
            }
        }
    }
    result
}

#[allow(clippy::too_many_arguments)]
pub fn redraw(
    mut commands: Commands,
    game: Res<Game>,
    mut view: ResMut<ViewState>,
    time: Res<Time>,
    window: Single<&Window, With<PrimaryWindow>>,
    mut handles: ResMut<MapMeshes>,
    mut meshes: ResMut<Assets<Mesh>>,
    markers: Query<Entity, With<Marker>>,
) {
    if view.insights_open {
        return;
    }
    if !view.dirty
        && (game.world.day == view.last_day || time.elapsed_secs_f64() - view.last_draw < 0.15)
    {
        return;
    }
    let world = &game.world;
    if handles.revision != view.snapshot_revision {
        handles.coarse = Grid::new(world.config.resolution.min(28)).expect("validated grid");
        handles.coarse_cells = (0..handles.coarse.len() as u32)
            .map(|i| world.grid.nearest(handles.coarse.direction(i)))
            .collect();
        handles.revision = view.snapshot_revision;
        handles.base_day = u64::MAX;
    }
    if handles.base_day != world.day || handles.base_layer != view.layer.label() {
        let mut base = Geometry::default();
        for (id, &sample) in handles.coarse_cells.iter().enumerate() {
            let rgb = color(&world.cells[sample as usize], view.layer);
            base.cell(
                &handles.coarse,
                id as u32,
                PLANET_RADIUS,
                rgb.map(|v| v * 0.72),
                1.,
            );
            base.cell(
                &handles.coarse,
                id as u32,
                PLANET_RADIUS + 0.006,
                rgb,
                0.965,
            );
        }
        *meshes.get_mut(&handles.base).unwrap() = base.mesh();
        handles.base_day = world.day;
        handles.base_layer = view.layer.label();
    }
    let spacing = cell_spacing(world);
    let pixels = spacing / (view.distance - PLANET_RADIUS) * window.height()
        / (2. * (CAMERA_FOV * 0.5).tan());
    let detailed = world.config.resolution <= 28 || pixels > 8.;
    view.lod = if detailed {
        1
    } else {
        world.config.resolution.div_ceil(28) as usize
    };
    let axis = view.orbit * Vec3::Z;
    let half_diagonal = ((CAMERA_FOV * 0.5).tan()
        * (1. + (window.width() / window.height().max(1.)).powi(2)).sqrt())
    .atan();
    let horizon = (PLANET_RADIUS / view.distance).acos();
    let cap = if view.distance * half_diagonal.sin() >= PLANET_RADIUS {
        horizon
    } else {
        (view.distance / PLANET_RADIUS * half_diagonal.sin()).asin() - half_diagonal
    };
    let threshold = (cap + spacing / PLANET_RADIUS * 3.)
        .min(std::f32::consts::FRAC_PI_2)
        .cos();
    let visible = if detailed {
        visible_cells(&world.grid, axis, threshold, 24_000)
    } else {
        Vec::new()
    };
    if visible.len() == 24_000 {
        view.lod = 2;
    }
    let mut detail = Geometry::default();
    let mut rivers = Geometry::default();
    for entity in &markers {
        commands.entity(entity).despawn();
    }
    let mut decoration_count = 0;
    for &id in &visible {
        let c = &world.cells[id as usize];
        let p = world.grid.direction(id);
        if world.config.resolution > 28 {
            let rgb = color(c, view.layer);
            detail.cell(
                &world.grid,
                id,
                PLANET_RADIUS + 0.03,
                rgb.map(|v| v * 0.72),
                1.,
            );
            detail.cell(&world.grid, id, PLANET_RADIUS + 0.04, rgb, 0.965);
        }
        if c.river > 0.
            && let Some(to) = c.drainage
        {
            rivers.line(
                p,
                world.grid.direction(to),
                PLANET_RADIUS + 0.065,
                spacing * (0.025 + c.river.ln() * 0.012).min(0.15),
                [0.35, 0.73, 0.86],
            );
        }
        if pixels > 22.
            && decoration_count < 500
            && id.is_multiple_of(3)
            && matches!(view.layer, Layer::Biomes)
            && p.dot(axis) > PLANET_RADIUS / view.distance + 0.015
        {
            let material = if c.resources.aquatic.total() > 1200. {
                Some(&handles.fish)
            } else if c.elevation > 1700 {
                Some(&handles.mountain)
            } else if c.biome.woodland() {
                Some(&handles.forest)
            } else {
                None
            };
            if let Some(material) = material {
                spawn_icon(
                    &mut commands,
                    &handles.quad,
                    material,
                    p * (PLANET_RADIUS + spacing * 0.12),
                    view.orbit,
                    spacing * 0.42,
                );
                decoration_count += 1;
            }
        }
    }
    if let Some(id) = view
        .selected
        .filter(|&id| (id as usize) < world.cells.len())
    {
        let corners: Vec<_> = world.grid.corners(id).collect();
        for i in 0..corners.len() {
            rivers.line(
                corners[i],
                corners[(i + 1) % corners.len()],
                PLANET_RADIUS + 0.09,
                spacing * 0.055,
                [1., 0.88, 0.49],
            );
        }
    }
    let mut count = 0;
    let mut fauna_cells = std::collections::BTreeMap::<CellId, bool>::new();
    for h in world.herds.iter().filter(|h| h.animals > 0) {
        *fauna_cells.entry(h.cell).or_default() |= !h.species.small();
    }
    let show_small_fauna =
        matches!(view.layer, Layer::Food) || view.distance - PLANET_RADIUS < spacing * 6.;
    for (cell, material, side, scale) in world
        .sites
        .iter()
        .map(|s| {
            (
                s.cell,
                if s.kind == humanity_core::SiteKind::FishingSettlement {
                    &handles.fishing_site
                } else {
                    &handles.seasonal_site
                },
                2.5,
                1.,
            )
        })
        .chain(
            world
                .tribes
                .iter()
                .map(|t| (t.cell, &handles.tribe, -1., 1.)),
        )
        .chain(
            fauna_cells
                .into_iter()
                .filter(|(_, large)| *large || show_small_fauna)
                .map(|(cell, large)| {
                    (
                        cell,
                        if large {
                            &handles.herd
                        } else {
                            &handles.small_fauna
                        },
                        1.,
                        0.5,
                    )
                }),
        )
    {
        if count >= 2200 {
            break;
        }
        let p = world.grid.direction(cell);
        if p.dot(axis) < (PLANET_RADIUS / view.distance + 0.02).max(threshold) {
            continue;
        }
        let size = (spacing * 0.55)
            .max((view.distance - PLANET_RADIUS) * 0.008)
            .min(spacing * 1.2);
        let offset = view.orbit * Vec3::X * spacing * 0.16 * side;
        spawn_icon(
            &mut commands,
            &handles.quad,
            material,
            p * (PLANET_RADIUS + size * 0.35) + offset,
            view.orbit,
            size * scale,
        );
        count += 1;
    }
    *meshes.get_mut(&handles.detail).unwrap() = detail.mesh();
    *meshes.get_mut(&handles.rivers).unwrap() = rivers.mesh();
    view.dirty = false;
    view.last_day = world.day;
    view.last_draw = time.elapsed_secs_f64();
}

fn spawn_icon(
    commands: &mut Commands,
    mesh: &Handle<Mesh>,
    material: &Handle<StandardMaterial>,
    p: Vec3,
    rotation: Quat,
    size: f32,
) {
    commands.spawn((
        Marker,
        Mesh3d(mesh.clone()),
        MeshMaterial3d(material.clone()),
        Transform {
            translation: p,
            rotation,
            scale: Vec3::splat(size),
        },
    ));
}

#[cfg(test)]
mod tests {
    use super::*;
    use bevy::ecs::system::RunSystemOnce;
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
    fn sphere_picking_uses_the_front_surface_at_every_cell_including_pentagons() {
        let w = world();
        let size = Vec2::new(1440., 900.);
        for id in 0..w.grid.len() as u32 {
            let q = Quat::from_rotation_arc(Vec3::Z, w.grid.direction(id));
            let p = screen_to_sphere(size * 0.5, size, q, PLANET_RADIUS * 3.).unwrap();
            assert_eq!(w.grid.nearest(p), id);
        }
        assert!(screen_to_sphere(Vec2::ZERO, size, Quat::IDENTITY, PLANET_RADIUS * 3.).is_none());
        assert!(ray_sphere(Vec3::Z * 300., Vec3::Z).is_none());
        let orbit = Quat::from_rotation_y(0.7) * Quat::from_rotation_x(-0.4);
        let origin = orbit * Vec3::Z * PLANET_RADIUS * 3.;
        let tan = (CAMERA_FOV * 0.5).tan();
        for id in 0..w.grid.len() as u32 {
            let direction = w.grid.direction(id);
            if direction.dot(origin.normalize()) < 0.5 {
                continue;
            }
            let p = orbit.conjugate() * (direction * PLANET_RADIUS - origin);
            let ndc = Vec2::new(p.x / (-p.z * tan * size.x / size.y), p.y / (-p.z * tan));
            let cursor = Vec2::new((ndc.x + 1.) * 0.5 * size.x, (1. - ndc.y) * 0.5 * size.y);
            let picked = screen_to_sphere(cursor, size, orbit, PLANET_RADIUS * 3.).unwrap();
            assert_eq!(w.grid.nearest(picked), id, "off-center perspective pick");
        }
    }
    #[test]
    fn follow_uses_stable_ids_and_crosses_poles_without_a_camera_singularity() {
        let world = world();
        let id = world.tribes[0].id;
        let mut app = App::new();
        app.insert_resource(Game {
            world,
            simulation: Simulation::default(),
        });
        app.insert_resource(ViewState {
            following: Some(id),
            distance: PLANET_RADIUS * 1.2,
            orbit: Quat::IDENTITY,
            ..default()
        });
        let camera = app
            .world_mut()
            .spawn((Camera3d::default(), Transform::default()))
            .id();
        for direction in [Vec3::Y, Vec3::NEG_Y, Vec3::NEG_Z, Vec3::Z] {
            let cell = {
                let mut game = app.world_mut().resource_mut::<Game>();
                let cell = game.world.grid.nearest(direction);
                game.world
                    .tribes
                    .iter_mut()
                    .find(|t| t.id == id)
                    .unwrap()
                    .cell = cell;
                game.world.tribes.reverse();
                cell
            };
            app.world_mut().run_system_once(orbit_camera).unwrap();
            let actual = app.world().get::<Transform>(camera).unwrap();
            let expected = app.world().resource::<Game>().world.grid.direction(cell);
            assert!(actual.translation.normalize().distance(expected) < 1e-5);
            assert!(actual.rotation.is_finite());
            assert_eq!(app.world().resource::<ViewState>().selected, Some(cell));
        }
        app.world_mut()
            .resource_mut::<Game>()
            .world
            .tribes
            .retain(|t| t.id != id);
        app.world_mut().run_system_once(orbit_camera).unwrap();
        assert_eq!(app.world().resource::<ViewState>().following, None);
    }
}
