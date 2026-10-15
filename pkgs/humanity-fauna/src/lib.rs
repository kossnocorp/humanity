//! Aggregate wildlife populations, biological traits and resource-limited ecology.
//! No human, renderer or World dependency. Only generation creates animals ex nihilo.
pub mod ecology;
pub mod hunting;
pub mod species;
pub use ecology::{Habitat, advance, seed};
use serde::{Deserialize, Serialize};
pub use species::{Diet, SPECIES_COUNT, Species, Traits};

/// Randomness supplied by the world's persisted stream.
pub trait FaunaRng {
    fn unit(&mut self) -> f32;
    fn expected(&mut self, mean: f32) -> u32 {
        mean.floor() as u32 + u32::from(self.unit() < mean.fract())
    }
}

/// A local wildlife population, herd, flock or colony, rather than an individual animal.
#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct Herd {
    /// Monotonic identity, never reused after death or a group merger.
    pub id: u32,
    /// Current spherical cell.
    pub cell: u32,
    /// Biological/ecological strategy.
    pub species: Species,
    /// Living animals including implicit juvenile/nonbreeding age classes.
    pub animals: u32,
    /// Nutritional condition, 0..1.
    pub condition: f32,
    /// Available movement distance in kilometers.
    pub travel: f32,
    /// Learned avoidance of humans after disturbance/hunting, 0..1.
    pub wariness: f32,
}

/// Persisted accounting by species: live = initial + births - natural deaths - harvest.
#[derive(Clone, Debug, Default, Serialize, Deserialize)]
pub struct FaunaStats {
    /// Animals placed during world generation only.
    pub initial: [u64; SPECIES_COUNT],
    /// Recruits produced by living populations.
    pub births: [u64; SPECIES_COUNT],
    /// Non-human deaths from background hazards and nutritional stress.
    pub natural_deaths: [u64; SPECIES_COUNT],
    /// Actual animals removed by hunting and trapping.
    pub harvested: [u64; SPECIES_COUNT],
    /// Population transfers that founded a daughter group; no animals created.
    pub dispersals: u64,
    /// Same-species populations consolidated in the same cell.
    pub mergers: u64,
}

#[cfg(test)]
mod tests;
