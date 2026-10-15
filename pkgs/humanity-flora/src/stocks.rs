use serde::{Deserialize, Serialize};

/// Three food channels shared by the entire local wildlife community.
#[derive(Clone, Copy, Debug, Default, Serialize, Deserialize, PartialEq)]
pub struct Forage {
    /// Grass/low-vegetation units (stocks or daily flow, according to context).
    pub grazing: f32,
    /// Woody leaves/twigs.
    pub browse: f32,
    /// Edible fruit/seeds, also available to human gatherers.
    pub fruit: f32,
}
impl Forage {
    pub fn new(grazing: f32, browse: f32, fruit: f32) -> Self {
        Self {
            grazing,
            browse,
            fruit,
        }
    }
    pub fn values(self) -> [f32; 3] {
        [self.grazing, self.browse, self.fruit]
    }
    pub fn scaled(self, factor: f32) -> Self {
        Self::new(
            self.grazing * factor,
            self.browse * factor,
            self.fruit * factor,
        )
    }
    pub fn total(self) -> f32 {
        self.grazing + self.browse + self.fruit
    }
}
impl std::ops::Add for Forage {
    type Output = Self;
    fn add(self, rhs: Self) -> Self {
        Self::new(
            self.grazing + rhs.grazing,
            self.browse + rhs.browse,
            self.fruit + rhs.fruit,
        )
    }
}
impl std::ops::Sub for Forage {
    type Output = Self;
    fn sub(self, rhs: Self) -> Self {
        Self::new(
            (self.grazing - rhs.grazing).max(0.),
            (self.browse - rhs.browse).max(0.),
            (self.fruit - rhs.fruit).max(0.),
        )
    }
}

/// Persistent vegetation. Herbivory/harvesting removes stocks; growth sees the
/// remaining biomass next day, so repeated overuse impairs future production.
#[derive(Clone, Debug, Default, Serialize, Deserialize)]
pub struct Vegetation {
    /// Edible roots/leaves in adult person-days.
    pub plants: f32,
    /// Edible fruit/seeds in adult person-days.
    pub fruit: f32,
    /// Low-vegetation forage stock in animal-food units.
    pub grazing: f32,
    /// Woody browse stock in animal-food units.
    pub browse: f32,
    /// Available craft wood units.
    pub wood: f32,
    /// Available craft fiber units.
    pub fiber: f32,
    /// Grass/browse vigor and derived fruit-bearing vigor, 0..1.
    pub vigor: [f32; 3],
    /// Current climate/community-dependent forage biomass capacities.
    pub capacity: Forage,
    /// Season-smoothed sustainable daily food flow, bounded by standing reserves
    /// and shared across species. This is carrying capacity, not today's net growth.
    pub wildlife_supply: Forage,
    /// Actual grass cover fraction after plant stress.
    pub grass_cover: f32,
    /// Actual woody cover fraction after plant stress.
    pub woody_cover: f32,
}
