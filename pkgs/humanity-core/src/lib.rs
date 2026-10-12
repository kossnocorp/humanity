//! Compact, renderer-independent state. One cell is an area; people are cohorts.
use serde::{Deserialize, Serialize};
mod culture;
mod insights;
mod sites;
pub use culture::*;
pub use insights::*;
pub use sites::*;

pub type CellId = u32;
pub const SAVE_VERSION: u32 = 5;
pub const MAX_CELLS: u64 = 4_000_000;

#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct PlanetConfig {
    /// Icosahedron edge subdivision frequency; cells = 10 * resolution^2 + 2.
    pub resolution: u32,
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
        if !(2..=humanity_grid::MAX_RESOLUTION).contains(&self.resolution) {
            return Err(format!(
                "resolution must be 2..={} (at most {MAX_CELLS} cells)",
                humanity_grid::MAX_RESOLUTION
            ));
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
        (10 * u64::from(self.resolution).pow(2) + 2) as usize
    }
    pub fn is_empty(&self) -> bool {
        false
    }
    pub fn year_days(&self) -> u32 {
        self.season_days * 4
    }
    /// Equal-area hex approximation relates mean neighbor spacing to radius.
    pub fn radius_km(&self) -> f32 {
        self.cell_km * (self.len() as f32 * 3_f32.sqrt() / (8. * std::f32::consts::PI)).sqrt()
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
    #[serde(default)]
    pub aquatic: AquaticLife,
    #[serde(default)]
    pub wood: f32,
    #[serde(default)]
    pub fiber: f32,
    /// Accessible toolstone availability, 0..1 (outcrop, not a renewable food).
    #[serde(default)]
    pub toolstone: f32,
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

#[derive(Clone, Copy, Debug, PartialEq, Eq, Serialize, Deserialize)]
#[repr(usize)]
pub enum Species {
    Reindeer,
    Bison,
    Antelope,
    Deer,
    Boar,
    Ibex,
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
    #[serde(default)]
    pub knowledge: Knowledge,
    #[serde(default)]
    pub equipment: Equipment,
    /// Today's fraction of food-working time assigned to aquatic foraging.
    #[serde(default)]
    pub fishing_effort: f32,
    #[serde(default)]
    pub camp_tradition: CampTradition,
    #[serde(default)]
    pub camp_pressure_days: u32,
    #[serde(default)]
    pub parent_band: Option<u32>,
    #[serde(default)]
    pub culture: u32,
    #[serde(default)]
    pub isolation_days: u32,
    #[serde(default)]
    pub last_cultural_contact: u64,
    #[serde(default)]
    pub contact_culture: Option<u32>,
    #[serde(default)]
    pub contact_days: u32,
    #[serde(default)]
    pub last_foreign_contact: u64,
    #[serde(default)]
    pub residence_days: u32,
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
    Fishing,
    Innovation,
    KnowledgeExchange,
    WaterCrossing,
    BandFission,
    CultureDivergence,
    CultureMixing,
    SiteFounded,
    SiteAbandoned,
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
    #[serde(default)]
    pub fished: f64,
    #[serde(default)]
    pub innovations: u64,
    #[serde(default)]
    pub knowledge_exchanges: u64,
    #[serde(default)]
    pub water_crossings: u64,
    #[serde(default)]
    pub band_fissions: u64,
    #[serde(default)]
    pub culture_divergences: u64,
    #[serde(default)]
    pub culture_mixtures: u64,
    #[serde(default)]
    pub sites_founded: u64,
    #[serde(default)]
    pub sites_abandoned: u64,
}

#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct World {
    pub version: u32,
    pub config: PlanetConfig,
    pub grid: humanity_grid::Grid,
    pub day: u64,
    pub rng: Rng,
    pub cells: Vec<Cell>,
    pub herds: Vec<Herd>,
    pub tribes: Vec<Tribe>,
    /// Monotonic IDs are never reused, even after a band dies out.
    #[serde(default)]
    pub next_band_id: u64,
    #[serde(default)]
    pub cultures: Vec<Culture>,
    #[serde(default)]
    pub sites: Vec<CulturalSite>,
    #[serde(default)]
    pub next_site_id: u64,
    /// Bounded recent history; lifetime counters remain in totals.
    pub events: std::collections::VecDeque<Event>,
    pub totals: Totals,
    #[serde(default)]
    pub insights: PopulationInsights,
}
impl World {
    /// A modelling threshold for mobile residential camps, not a biological cap.
    pub fn camp_limit(&self, t: &Tribe) -> u32 {
        let n = t.headcount().max(1) as f32;
        let mut limit = t.camp_tradition.normal_limit();
        if t.camp_tradition == CampTradition::RiverBudding
            && t.knowledge.level(Skill::Fishing) >= SOCIAL.rich_fishing
            && t.knowledge.level(Skill::Preservation) >= SOCIAL.rich_preservation
            && t.food / n >= SOCIAL.rich_food_days
        {
            let aquatic: f32 = std::iter::once(t.cell)
                .chain(self.grid.neighbors(t.cell))
                .map(|id| &self.cells[id as usize])
                .filter(|c| c.biome != Biome::Ocean)
                .map(|c| c.resources.aquatic.total() * (1. - c.weather.snow * 0.9))
                .sum();
            if aquatic >= n * SOCIAL.rich_aquatic_days {
                limit = SOCIAL.rich_camp_limit;
            }
        }
        if t.food / n < SOCIAL.scarce_food_days || t.health < SOCIAL.scarce_health {
            limit = (limit as f32 * SOCIAL.scarce_limit_fraction) as u32;
        }
        limit
    }

    pub fn local_season(&self, cell: CellId) -> u32 {
        ((self.day / u64::from(self.config.season_days)
            + if self.grid.latitude(cell) < 0. { 2 } else { 0 })
            % 4) as u32
    }

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
