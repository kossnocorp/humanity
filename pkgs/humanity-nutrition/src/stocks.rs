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
    /// Edible plant stock in person-days.
    pub plants: f32,
    /// Edible fruit stock in person-days.
    pub fruit: f32,
    /// Edible small-game stock in person-days.
    pub game: f32,
    /// Grazing stock in animal-days.
    pub grazing: f32,
    /// Renewable aquatic food stocks.
    pub aquatic: AquaticLife,
    /// Available craft wood units.
    pub wood: f32,
    /// Available plant-fiber units.
    pub fiber: f32,
    /// Accessible toolstone availability, 0..1.
    pub toolstone: f32,
}
impl Resources {
    pub fn edible(&self) -> f32 {
        self.plants + self.fruit + self.game
    }
    pub fn harvested(&mut self, food: f32, area: f32) {
        self.foraging_pressure += food / (area * crate::FORAGING.patch_harvest_scale);
    }
}
