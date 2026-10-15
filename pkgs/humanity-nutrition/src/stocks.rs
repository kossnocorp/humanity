use humanity_flora::Vegetation;
use serde::{Deserialize, Serialize};

#[derive(Clone, Debug, Default, Serialize, Deserialize)]
pub struct AquaticLife {
    /// Freshwater edible standing biomass in person-days.
    pub freshwater_fish: f32,
    /// Coastal/pelagic edible standing biomass in person-days.
    pub coastal_fish: f32,
    /// Shellfish edible standing biomass in person-days.
    pub shellfish: f32,
}
impl AquaticLife {
    pub fn total(&self) -> f32 {
        self.freshwater_fish + self.coastal_fish + self.shellfish
    }
}
#[derive(Clone, Debug, Default, Serialize, Deserialize)]
pub struct Resources {
    /// Depletion of easily reached patches; remote standing biomass remains.
    pub foraging_pressure: f32,
    /// Plant stocks, condition and production owned by humanity-flora.
    pub vegetation: Vegetation,
    /// Derived small-wildlife biomass in person-days; harvesting must remove actual fauna.
    pub game: f32,
    /// Renewable aquatic food stocks.
    pub aquatic: AquaticLife,
    /// Accessible toolstone availability, 0..1.
    pub toolstone: f32,
}
// Preserve convenient stock access for consumers; serialization keeps vegetation
// explicitly nested so ownership and production state remain inspectable.
impl std::ops::Deref for Resources {
    type Target = Vegetation;
    fn deref(&self) -> &Vegetation {
        &self.vegetation
    }
}
impl std::ops::DerefMut for Resources {
    fn deref_mut(&mut self) -> &mut Vegetation {
        &mut self.vegetation
    }
}
impl Resources {
    pub fn edible(&self) -> f32 {
        self.plants + self.fruit + self.game
    }
    pub fn harvested(&mut self, food: f32, area: f32) {
        self.foraging_pressure += food / (area * crate::FORAGING.patch_harvest_scale);
    }
}
