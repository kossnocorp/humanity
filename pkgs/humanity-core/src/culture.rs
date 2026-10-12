//! Practical traditions are distributed among age cohorts, not global unlocks.
use crate::{Biome, Cell, Species};
use serde::{Deserialize, Serialize};

pub use humanity_cultures::{CampTradition, CultureVariant, Residence, SOCIAL};

#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct Culture {
    /// Stable index in World::cultures. Extinct identities remain as ancestry.
    pub id: u32,
    pub name: String,
    pub variant: CultureVariant,
    pub parents: Vec<u32>,
    pub founded: u64,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq, Serialize, Deserialize)]
#[repr(usize)]
pub enum Skill {
    Knapping,
    Cordage,
    Fishing,
    Watercraft,
    Tracking,
    DriveHunting,
    Trapping,
    PlantProcessing,
    Preservation,
    Clothing,
    Healing,
    OralTradition,
    Navigation,
}
impl Skill {
    pub const ALL: [Self; 13] = [
        Self::Knapping,
        Self::Cordage,
        Self::Fishing,
        Self::Watercraft,
        Self::Tracking,
        Self::DriveHunting,
        Self::Trapping,
        Self::PlantProcessing,
        Self::Preservation,
        Self::Clothing,
        Self::Healing,
        Self::OralTradition,
        Self::Navigation,
    ];
    pub fn label(self) -> &'static str {
        match self {
            Self::Knapping => "Stone working",
            Self::Cordage => "Cordage / baskets",
            Self::Fishing => "Hooks / nets / spears",
            Self::Watercraft => "Rafts / canoes",
            Self::Tracking => "Animal tracking",
            Self::DriveHunting => "Cooperative hunts",
            Self::Trapping => "Small-game traps",
            Self::PlantProcessing => "Roots / seeds / nuts",
            Self::Preservation => "Drying / smoking",
            Self::Clothing => "Hide working / sewing",
            Self::Healing => "Care / plant remedies",
            Self::OralTradition => "Oral teaching",
            Self::Navigation => "Water navigation",
        }
    }
}

#[derive(Clone, Debug, Default, Serialize, Deserialize)]
pub struct Knowledge {
    /// Fraction of children, adults, elders proficient in each tradition.
    /// Overlapping skills: these fractions do not partition the population.
    pub practitioners: [[f32; 3]; 13],
    /// Shared species-specific hunting familiarity, learned through practice/contact.
    pub prey: [f32; 6],
}
impl Knowledge {
    pub fn level(&self, skill: Skill) -> f32 {
        self.practitioners[skill as usize][1]
    }
    pub fn seed(&mut self, skill: Skill, level: f32) {
        self.practitioners[skill as usize] = [level * 0.35, level, (level * 1.2).min(1.)];
    }
    pub fn prerequisites(&self, skill: Skill) -> bool {
        match skill {
            Skill::Watercraft => {
                self.level(Skill::Cordage) > 0.12 && self.level(Skill::Knapping) > 0.12
            }
            Skill::Navigation => self.level(Skill::Watercraft) > 0.12,
            Skill::Fishing | Skill::Trapping | Skill::Clothing => self.level(Skill::Cordage) > 0.08,
            Skill::DriveHunting => self.level(Skill::Tracking) > 0.12,
            _ => true,
        }
    }
}

#[derive(Clone, Debug, Default, Serialize, Deserialize)]
pub struct Equipment {
    /// Abstract portable material units; watercraft capacity is passenger places.
    pub wood: f32,
    pub fiber: f32,
    pub stone: f32,
    pub hides: f32,
    pub boat_places: f32,
}

#[derive(Clone, Debug, Default, Serialize, Deserialize)]
pub struct AquaticLife {
    /// Standing edible biomass in person-days, not individual fish counts.
    pub freshwater_fish: f32,
    pub coastal_fish: f32,
    pub shellfish: f32,
}
impl AquaticLife {
    pub fn total(&self) -> f32 {
        self.freshwater_fish + self.coastal_fish + self.shellfish
    }
}

impl Species {
    pub fn annual_breeding(self) -> f32 {
        match self {
            Self::Boar => 0.65,
            Self::Deer => 0.35,
            Self::Antelope => 0.4,
            Self::Ibex => 0.23,
            Self::Bison => 0.22,
            Self::Reindeer => 0.28,
        }
    }
    pub fn grazing_need(self) -> f32 {
        match self {
            Self::Bison => 1.8,
            Self::Reindeer => 0.9,
            Self::Antelope => 0.7,
            Self::Deer => 0.8,
            Self::Boar => 0.65,
            Self::Ibex => 0.55,
        }
    }
    pub const ALL: [Self; 6] = [
        Self::Reindeer,
        Self::Bison,
        Self::Antelope,
        Self::Deer,
        Self::Boar,
        Self::Ibex,
    ];
    pub fn index(self) -> usize {
        self as usize
    }
    pub fn meat(self) -> f32 {
        match self {
            Self::Bison => 100.,
            Self::Reindeer => 45.,
            Self::Antelope => 35.,
            Self::Deer => 40.,
            Self::Boar => 30.,
            Self::Ibex => 25.,
        }
    }
    pub fn ideal_temperature(self) -> f32 {
        match self {
            Self::Reindeer => 2.,
            Self::Bison => 14.,
            Self::Antelope => 26.,
            Self::Deer => 12.,
            Self::Boar => 20.,
            Self::Ibex => 6.,
        }
    }
    pub fn habitat(self, c: &Cell) -> f32 {
        let forest = matches!(c.biome, Biome::Forest | Biome::Taiga | Biome::Rainforest);
        match self {
            Self::Deer | Self::Boar if forest => 14.,
            Self::Ibex if c.elevation > 1000 => 18.,
            Self::Bison | Self::Antelope if forest => -12.,
            Self::Deer | Self::Boar if !forest => -8.,
            _ => 0.,
        }
    }
    pub fn difficulty(self) -> f32 {
        match self {
            Self::Bison => 0.18,
            Self::Boar => 0.12,
            Self::Ibex => 0.15,
            _ => 0.05,
        }
    }
}
