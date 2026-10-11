//! Compact, renderer-independent state. One cell is an area; people are cohorts.
use serde::{Deserialize, Serialize};

pub type CellId = u32;
pub const SAVE_VERSION: u32 = 1;
pub const MAX_CELLS: u64 = 4_000_000;

#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct PlanetConfig {
    pub width: u32,
    pub height: u32,
    pub cell_km: f32,
    /// Annual sea-level equatorial temperature, Celsius.
    pub temperature: f32,
    pub season_days: u32,
    pub axial_tilt: f32,
    pub ocean_fraction: f32,
    pub rainfall: f32,
    pub population: u32,
    pub seed: u64,
}

impl PlanetConfig {
    pub fn validate(&self) -> Result<(), String> {
        if self.width < 24 || self.height < 16 || !self.height.is_multiple_of(2) {
            return Err("width >= 24 and even height >= 16 are required".into());
        }
        if u64::from(self.width) * u64::from(self.height) > MAX_CELLS {
            return Err(format!("planet exceeds {MAX_CELLS} cells"));
        }
        for (name, value, min, max) in [
            ("cell-km", self.cell_km, 1., 200.),
            ("temperature", self.temperature, -30., 60.),
            ("axial-tilt", self.axial_tilt, 0., 60.),
            ("ocean-fraction", self.ocean_fraction, 0.1, 0.9),
            ("rainfall", self.rainfall, 0.05, 4.),
        ] {
            if !value.is_finite() || !(min..=max).contains(&value) {
                return Err(format!("{name} must be finite and in {min}..={max}"));
            }
        }
        if !(4..=1000).contains(&self.season_days) {
            return Err("season-days must be in 4..=1000".into());
        }
        if !(1..=100_000_000).contains(&self.population) {
            return Err("population must be in 1..=100000000".into());
        }
        Ok(())
    }

    pub fn len(&self) -> usize {
        (self.width * self.height) as usize
    }
    pub fn is_empty(&self) -> bool {
        self.width == 0 || self.height == 0
    }
    pub fn year_days(&self) -> u32 {
        self.season_days * 4
    }
    pub fn latitude(&self, cell: CellId) -> f32 {
        1. - 2. * ((cell / self.width) as f32 + 0.5) / self.height as f32
    }
    /// Odd-row offset hexes on a cylinder: wrap longitude, bound the poles.
    pub fn neighbors(&self, cell: CellId) -> impl Iterator<Item = CellId> + '_ {
        let x = (cell % self.width) as i32;
        let y = (cell / self.width) as i32;
        let diagonal = if y % 2 == 0 { -1 } else { 1 };
        [
            (1, 0),
            (-1, 0),
            (0, -1),
            (diagonal, -1),
            (0, 1),
            (diagonal, 1),
        ]
        .into_iter()
        .filter_map(move |(dx, dy)| {
            let row = y + dy;
            (row >= 0 && row < self.height as i32)
                .then(|| row as u32 * self.width + (x + dx).rem_euclid(self.width as i32) as u32)
        })
    }
}

/// SplitMix64, including its exact resumable state. No platform-dependent RNG.
#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct Rng {
    pub state: u64,
}
impl Rng {
    pub fn new(seed: u64) -> Self {
        Self { state: seed }
    }
    pub fn next_u64(&mut self) -> u64 {
        self.state = self.state.wrapping_add(0x9e3779b97f4a7c15);
        let mut z = self.state;
        z = (z ^ (z >> 30)).wrapping_mul(0xbf58476d1ce4e5b9);
        z = (z ^ (z >> 27)).wrapping_mul(0x94d049bb133111eb);
        z ^ (z >> 31)
    }
    pub fn unit(&mut self) -> f32 {
        (self.next_u64() >> 40) as f32 / 16_777_216.
    }
    pub fn chance(&mut self, p: f32) -> bool {
        self.unit() < p.clamp(0., 1.)
    }
    pub fn index(&mut self, len: usize) -> usize {
        (self.next_u64() % len as u64) as usize
    }
    pub fn expected(&mut self, mean: f32) -> u32 {
        mean.floor() as u32 + u32::from(self.chance(mean.fract()))
    }
}

#[derive(Clone, Copy, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub enum Biome {
    Ocean,
    Sea,
    Lake,
    Ice,
    Tundra,
    Taiga,
    Grassland,
    Forest,
    Rainforest,
    Savanna,
    Desert,
    Alpine,
}
impl Biome {
    pub fn water(self) -> bool {
        matches!(self, Self::Ocean | Self::Sea | Self::Lake)
    }
    pub fn fertility(self) -> f32 {
        match self {
            Self::Ocean | Self::Sea | Self::Lake => 0.,
            Self::Ice => 0.015,
            Self::Tundra => 0.16,
            Self::Taiga => 0.45,
            Self::Grassland => 0.8,
            Self::Forest => 0.85,
            Self::Rainforest => 1.,
            Self::Savanna => 0.65,
            Self::Desert => 0.06,
            Self::Alpine => 0.1,
        }
    }
}

#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct Weather {
    pub temperature: f32,
    pub precipitation: f32,
    pub snow: f32,
    pub soil_moisture: f32,
}

#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct Resources {
    /// Renewable standing stocks, in edible person-days except grazing (animal-days).
    pub plants: f32,
    pub fruit: f32,
    pub game: f32,
    pub grazing: f32,
}
impl Resources {
    pub fn edible(&self) -> f32 {
        self.plants + self.fruit + self.game
    }
}

#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct Cell {
    pub elevation: i32,
    pub biome: Biome,
    pub moisture: f32,
    pub drainage: Option<CellId>,
    /// Catchment-derived river flow (relative, not a claim of cubic m/s).
    pub river: f32,
    pub weather: Weather,
    pub resources: Resources,
}

#[derive(Clone, Copy, Debug, Serialize, Deserialize)]
pub enum Species {
    Reindeer,
    Bison,
    Antelope,
}
#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct Herd {
    pub id: u32,
    pub cell: CellId,
    pub species: Species,
    pub animals: u32,
    pub condition: f32,
    pub travel: f32,
}

#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct Language {
    pub family: u32,
    pub share: f32,
}

#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct Demography {
    /// Female / male cohorts: children (<15), adults (15–49), elders (50+).
    pub female: [u32; 3],
    pub male: [u32; 3],
}
impl Demography {
    pub fn total(&self) -> u32 {
        self.female.iter().chain(&self.male).sum()
    }
    pub fn adults(&self) -> u32 {
        self.female[1] + self.male[1]
    }
    pub fn remove(&mut self, count: u32, rng: &mut Rng) -> u32 {
        let count = count.min(self.total());
        // Six cohort operations, independent of population size.
        let mut remaining = count;
        let mut total = self.total();
        for cohort in self.female.iter_mut().chain(&mut self.male) {
            let n = if total == *cohort {
                remaining
            } else {
                rng.expected(remaining as f32 * *cohort as f32 / total.max(1) as f32)
                    .min(*cohort)
                    .min(remaining)
            };
            total -= *cohort;
            *cohort -= n;
            remaining -= n;
        }
        count - remaining
    }
}

#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct Specialists {
    pub hunters: u32,
    pub gatherers: u32,
    pub healers: u32,
    pub guides: u32,
}
impl Specialists {
    pub fn from_adults(n: u32) -> Self {
        let hunters = n * 4 / 10;
        let healers = n / 20;
        let guides = n / 10;
        Self {
            hunters,
            healers,
            guides,
            gatherers: n - hunters - healers - guides,
        }
    }
}

#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct Tribe {
    pub id: u32,
    pub cell: CellId,
    pub people: Demography,
    pub specialists: Specialists,
    pub languages: Vec<Language>,
    pub food: f32,
    pub health: f32,
    pub travel: f32,
    pub hunting_skill: f32,
}
impl Tribe {
    pub fn headcount(&self) -> u32 {
        self.people.total()
    }
}

pub fn understanding(a: &[Language], b: &[Language]) -> f32 {
    a.iter()
        .map(|x| {
            b.iter()
                .filter(|y| x.family == y.family)
                .map(|y| x.share.min(y.share))
                .sum::<f32>()
        })
        .sum::<f32>()
        .clamp(0., 1.)
}

#[derive(Clone, Copy, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub enum EventKind {
    Hunt,
    HuntFailed,
    Conflict,
    Exchange,
    Migration,
    Birth,
    Death,
    Extinction,
}
#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct Event {
    pub day: u64,
    pub kind: EventKind,
    pub cell: CellId,
    pub tribe: u32,
    pub other: Option<u32>,
    pub amount: u32,
    pub probability: f32,
}

#[derive(Clone, Default, Debug, Serialize, Deserialize)]
pub struct Totals {
    pub hunts: u64,
    pub failed_hunts: u64,
    pub animals_hunted: u64,
    pub conflicts: u64,
    pub exchanges: u64,
    pub migrations: u64,
    pub births: u64,
    pub deaths: u64,
    pub gathered: f64,
}

#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct World {
    pub version: u32,
    pub config: PlanetConfig,
    pub day: u64,
    pub rng: Rng,
    pub cells: Vec<Cell>,
    pub herds: Vec<Herd>,
    pub tribes: Vec<Tribe>,
    /// Bounded recent history; lifetime counters remain in totals.
    pub events: std::collections::VecDeque<Event>,
    pub totals: Totals,
}
impl World {
    pub fn population(&self) -> u64 {
        self.tribes.iter().map(|t| u64::from(t.headcount())).sum()
    }
    pub fn animals(&self) -> u64 {
        self.herds.iter().map(|h| u64::from(h.animals)).sum()
    }
    pub fn event(&mut self, event: Event) {
        if self.events.len() == 512 {
            self.events.pop_front();
        }
        self.events.push_back(event);
    }
    pub fn season(&self) -> &'static str {
        ["Spring", "Summer", "Autumn", "Winter"]
            [(self.day / u64::from(self.config.season_days) % 4) as usize]
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    pub fn config() -> PlanetConfig {
        PlanetConfig {
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
        }
    }
    #[test]
    fn neighbors_are_reciprocal_at_seams_and_poles() {
        let c = config();
        for a in 0..c.len() as u32 {
            let neighbors: Vec<_> = c.neighbors(a).collect();
            assert!(neighbors.len() >= 4 && neighbors.len() <= 6);
            for b in neighbors {
                assert!(c.neighbors(b).any(|n| n == a));
            }
        }
    }
    #[test]
    fn cohort_removal_conserves_population() {
        for count in 0..120 {
            let mut d = Demography {
                female: [10, 24, 5],
                male: [13, 32, 8],
            };
            let before = d.total();
            let n = d.remove(count, &mut Rng::new(count as u64));
            assert_eq!(n, count.min(before));
            assert_eq!(d.total(), before - n);
        }
    }
}
