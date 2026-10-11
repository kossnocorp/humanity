//! Dual of an icosahedral geodesic triangulation: 12 pentagons, all other
//! cells hexagons. Shared spherical circumcenters give watertight cell borders.
//! Cell IDs are deterministic; only resolution is persisted, not derived caches.
use glam::Vec3;
use serde::{Deserialize, Deserializer, Serialize, Serializer};
use std::{collections::HashMap, sync::Arc};

pub const MAX_RESOLUTION: u32 = 632;
const EMPTY: u32 = u32::MAX;

#[derive(Clone, Debug)]
struct Topology {
    neighbors: [u32; 6],
    corners: [u32; 6],
}
#[derive(Debug)]
struct Data {
    directions: Vec<Vec3>,
    corners: Vec<Vec3>,
    topology: Vec<Topology>,
    latitude: Vec<f32>,
    cooling: Vec<f32>,
    area_factor: Vec<f32>,
}
#[derive(Clone, Debug)]
pub struct Grid {
    resolution: u32,
    data: Arc<Data>,
}

impl Serialize for Grid {
    fn serialize<S: Serializer>(&self, serializer: S) -> Result<S::Ok, S::Error> {
        serializer.serialize_u32(self.resolution)
    }
}
impl<'de> Deserialize<'de> for Grid {
    fn deserialize<D: Deserializer<'de>>(deserializer: D) -> Result<Self, D::Error> {
        Self::new(u32::deserialize(deserializer)?).map_err(serde::de::Error::custom)
    }
}

impl Grid {
    pub fn new(resolution: u32) -> Result<Self, String> {
        if !(2..=MAX_RESOLUTION).contains(&resolution) {
            return Err(format!("resolution must be 2..={MAX_RESOLUTION}"));
        }
        let n = resolution;
        let t = (1. + 5_f32.sqrt()) / 2.;
        let mut directions = vec![
            Vec3::new(-1., t, 0.),
            Vec3::new(1., t, 0.),
            Vec3::new(-1., -t, 0.),
            Vec3::new(1., -t, 0.),
            Vec3::new(0., -1., t),
            Vec3::new(0., 1., t),
            Vec3::new(0., -1., -t),
            Vec3::new(0., 1., -t),
            Vec3::new(t, 0., -1.),
            Vec3::new(t, 0., 1.),
            Vec3::new(-t, 0., -1.),
            Vec3::new(-t, 0., 1.),
        ];
        for p in &mut directions {
            *p = p.normalize();
        }
        directions.reserve((10 * n * n + 2) as usize - 12);
        let faces = [
            [0, 11, 5],
            [0, 5, 1],
            [0, 1, 7],
            [0, 7, 10],
            [0, 10, 11],
            [1, 5, 9],
            [5, 11, 4],
            [11, 10, 2],
            [10, 7, 6],
            [7, 1, 8],
            [3, 9, 4],
            [3, 4, 2],
            [3, 2, 6],
            [3, 6, 8],
            [3, 8, 9],
            [4, 9, 5],
            [2, 4, 11],
            [6, 2, 10],
            [8, 6, 7],
            [9, 8, 1],
        ];
        let mut triangles = Vec::<[u32; 3]>::with_capacity((20 * n * n) as usize);
        let mut edges = HashMap::with_capacity((30 * (n - 1)) as usize);
        // Integer edge identities avoid floating-point deduplication at face seams.
        for [a, b, c] in faces {
            let mut local = Vec::with_capacity(((n + 1) * (n + 2) / 2) as usize);
            let index = |i: u32, j: u32| (i * (n + 1) - i * i.saturating_sub(1) / 2 + j) as usize;
            for i in 0..=n {
                for j in 0..=n - i {
                    let weights = [(a, n - i - j), (b, i), (c, j)];
                    if let Some(&(vertex, _)) = weights.iter().find(|&&(_, w)| w == n) {
                        local.push(vertex);
                        continue;
                    }
                    let edge = |v0: u32, v1: u32, w1: u32| {
                        if v0 < v1 {
                            (v0, v1, w1)
                        } else {
                            (v1, v0, n - w1)
                        }
                    };
                    let key = if n - i - j == 0 {
                        Some(edge(b, c, j))
                    } else if i == 0 {
                        Some(edge(a, c, j))
                    } else if j == 0 {
                        Some(edge(a, b, i))
                    } else {
                        None
                    };
                    if let Some(existing) = key.and_then(|key| edges.get(&key)).copied() {
                        local.push(existing);
                        continue;
                    }
                    let p = (directions[a as usize] * (n - i - j) as f32
                        + directions[b as usize] * i as f32
                        + directions[c as usize] * j as f32)
                        .normalize();
                    let id = directions.len() as u32;
                    directions.push(p);
                    local.push(id);
                    if let Some(key) = key {
                        edges.insert(key, id);
                    }
                }
            }
            for i in 0..n {
                for j in 0..n - i {
                    let [a, b, c] = [
                        local[index(i, j)],
                        local[index(i + 1, j)],
                        local[index(i, j + 1)],
                    ];
                    triangles.push([a, b, c]);
                    if j < n - i - 1 {
                        triangles.push([b, local[index(i + 1, j + 1)], c]);
                    }
                }
            }
        }
        let mut topology = vec![
            Topology {
                neighbors: [EMPTY; 6],
                corners: [EMPTY; 6]
            };
            directions.len()
        ];
        let mut corners = Vec::with_capacity(triangles.len());
        for [a, b, c] in triangles {
            let [pa, pb, pc] = [
                directions[a as usize],
                directions[b as usize],
                directions[c as usize],
            ];
            // Use double precision for small triangles on very high-resolution planets.
            let mut center = (pb.as_dvec3() - pa.as_dvec3())
                .cross(pc.as_dvec3() - pa.as_dvec3())
                .normalize()
                .as_vec3();
            if center.dot(pa) < 0. {
                center = -center;
            }
            let corner = corners.len() as u32;
            corners.push(center);
            for (id, neighbors) in [(a, [b, c]), (b, [a, c]), (c, [a, b])] {
                let entry = &mut topology[id as usize];
                *entry
                    .corners
                    .iter_mut()
                    .find(|v| **v == EMPTY)
                    .expect("degree <= 6") = corner;
                for neighbor in neighbors {
                    if !entry.neighbors.contains(&neighbor) {
                        *entry
                            .neighbors
                            .iter_mut()
                            .find(|v| **v == EMPTY)
                            .expect("degree <= 6") = neighbor;
                    }
                }
            }
        }
        for (id, cell) in topology.iter_mut().enumerate() {
            let normal = directions[id];
            let tangent = normal.any_orthonormal_vector();
            let bitangent = normal.cross(tangent);
            let count = if id < 12 { 5 } else { 6 };
            cell.corners[..count].sort_unstable_by(|&a, &b| {
                let angle = |i: u32| {
                    let p = corners[i as usize];
                    p.dot(bitangent).atan2(p.dot(tangent))
                };
                angle(a).total_cmp(&angle(b))
            });
        }
        let latitude: Vec<_> = directions
            .iter()
            .map(|p| p.y.clamp(-1., 1.).asin() / std::f32::consts::FRAC_PI_2)
            .collect();
        let cooling = latitude.iter().map(|v| v.abs().powf(1.35)).collect();
        let mean_area = 4. * std::f64::consts::PI / directions.len() as f64;
        let area_factor = topology
            .iter()
            .enumerate()
            .map(|(id, cell)| {
                let p = directions[id].as_dvec3();
                let count = if id < 12 { 5 } else { 6 };
                let area: f64 = (0..count)
                    .map(|i| {
                        let a = corners[cell.corners[i] as usize].as_dvec3();
                        let b = corners[cell.corners[(i + 1) % count] as usize].as_dvec3();
                        2. * p
                            .dot(a.cross(b))
                            .abs()
                            .atan2(1. + p.dot(a) + a.dot(b) + b.dot(p))
                    })
                    .sum();
                (area / mean_area) as f32
            })
            .collect();
        Ok(Self {
            resolution,
            data: Arc::new(Data {
                directions,
                corners,
                topology,
                latitude,
                cooling,
                area_factor,
            }),
        })
    }
    pub fn resolution(&self) -> u32 {
        self.resolution
    }
    pub fn len(&self) -> usize {
        self.data.directions.len()
    }
    pub fn is_empty(&self) -> bool {
        self.data.directions.is_empty()
    }
    pub fn direction(&self, cell: u32) -> Vec3 {
        self.data.directions[cell as usize]
    }
    /// Latitude normalized to -1..1, north is +Y.
    pub fn latitude(&self, cell: u32) -> f32 {
        self.data.latitude[cell as usize]
    }
    pub fn cooling(&self, cell: u32) -> f32 {
        self.data.cooling[cell as usize]
    }
    /// True spherical cell area divided by the mean cell area.
    pub fn area_factor(&self, cell: u32) -> f32 {
        self.data.area_factor[cell as usize]
    }
    pub fn longitude(&self, cell: u32) -> f32 {
        let p = self.direction(cell);
        p.z.atan2(p.x)
    }
    pub fn degree(&self, cell: u32) -> usize {
        if cell < 12 { 5 } else { 6 }
    }
    pub fn neighbors(&self, cell: u32) -> impl Iterator<Item = u32> + '_ {
        self.data.topology[cell as usize].neighbors[..self.degree(cell)]
            .iter()
            .copied()
    }
    pub fn corners(&self, cell: u32) -> impl Iterator<Item = Vec3> + '_ {
        self.data.topology[cell as usize].corners[..self.degree(cell)]
            .iter()
            .map(|&i| self.data.corners[i as usize])
    }
    pub fn angular_distance(&self, a: u32, b: u32) -> f32 {
        let x = self.direction(a);
        let y = self.direction(b);
        x.cross(y).length().atan2(x.dot(y))
    }
    /// Hill climb on the convex triangulation, starting with the best icosahedron
    /// vertex. No planet-wide scan or spatial hash allocation for picking.
    pub fn nearest(&self, direction: Vec3) -> u32 {
        let score = |id| self.direction(id).as_dvec3().dot(direction.as_dvec3());
        let mut cell = (0..12)
            .max_by(|&a, &b| score(a).total_cmp(&score(b)))
            .unwrap();
        loop {
            let mut best = cell;
            for n in self.neighbors(cell) {
                if score(n) > score(best) {
                    best = n;
                }
            }
            if best == cell {
                return cell;
            }
            cell = best;
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn sphere_is_closed_reciprocal_and_has_exactly_twelve_pentagons() {
        for n in [2, 3, 8, 28] {
            let g = Grid::new(n).unwrap();
            assert_eq!(g.len(), (10 * n * n + 2) as usize);
            let mut edges = 0;
            let mut pentagons = 0;
            for id in 0..g.len() as u32 {
                let count = g.neighbors(id).count();
                pentagons += usize::from(count == 5);
                assert_eq!(g.corners(id).count(), count);
                edges += count;
                assert!((g.direction(id).length() - 1.).abs() < 1e-5);
                for neighbor in g.neighbors(id) {
                    assert!(g.neighbors(neighbor).any(|v| v == id));
                    let common = g
                        .corners(id)
                        .filter(|a| g.corners(neighbor).any(|b| a.distance_squared(b) < 1e-12))
                        .count();
                    assert_eq!(common, 2, "adjacent cells must share exactly one edge");
                }
                let corners: Vec<_> = g.corners(id).collect();
                for i in 0..corners.len() {
                    assert!(
                        (corners[i] - g.direction(id))
                            .cross(corners[(i + 1) % corners.len()] - g.direction(id))
                            .dot(g.direction(id))
                            > 0.
                    );
                }
            }
            assert_eq!(pentagons, 12);
            let area: f64 = (0..g.len() as u32)
                .map(|id| f64::from(g.area_factor(id)))
                .sum();
            assert!(
                (area / g.len() as f64 - 1.).abs() < 1e-5,
                "cells cover the sphere exactly once"
            );
            assert_eq!(g.len() as i64 - (edges / 2) as i64 + (20 * n * n) as i64, 2);
        }
    }
    #[test]
    fn picking_matches_brute_force_and_round_trips_at_poles_and_seams() {
        let g = Grid::new(12).unwrap();
        for id in 0..g.len() as u32 {
            assert_eq!(g.nearest(g.direction(id)), id);
        }
        for i in 0..300 {
            let p = Vec3::new(
                (i as f32 * 1.72).sin(),
                (i as f32 * 0.83).cos(),
                (i as f32 * 3.17).sin(),
            )
            .normalize();
            let brute = (0..g.len() as u32)
                .max_by(|&a, &b| g.direction(a).dot(p).total_cmp(&g.direction(b).dot(p)))
                .unwrap();
            assert_eq!(g.nearest(p), brute);
        }
        let restored: Grid = serde_json::from_str(&serde_json::to_string(&g).unwrap()).unwrap();
        for id in 0..g.len() as u32 {
            assert_eq!(restored.direction(id), g.direction(id));
        }
    }

    #[test]
    fn dense_planet_picking_and_connectivity_remain_valid() {
        let g = Grid::new(128).unwrap();
        let mut seen = vec![false; g.len()];
        let mut queue = std::collections::VecDeque::from([0_u32]);
        seen[0] = true;
        while let Some(id) = queue.pop_front() {
            for n in g.neighbors(id) {
                if !seen[n as usize] {
                    seen[n as usize] = true;
                    queue.push_back(n);
                }
            }
        }
        assert!(seen.into_iter().all(|v| v));
        for i in 0..32 {
            let p = Vec3::new(
                (i as f32 * 1.73).sin(),
                (i as f32 * 0.83).cos(),
                (i as f32 * 3.17).sin(),
            )
            .normalize();
            let brute = (0..g.len() as u32)
                .max_by(|&a, &b| {
                    g.direction(a)
                        .as_dvec3()
                        .dot(p.as_dvec3())
                        .total_cmp(&g.direction(b).as_dvec3().dot(p.as_dvec3()))
                })
                .unwrap();
            assert_eq!(g.nearest(p), brute);
        }
    }
}
