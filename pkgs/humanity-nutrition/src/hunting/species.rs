use serde::{Deserialize, Serialize};

#[derive(Clone, Copy, Debug, PartialEq, Eq, Serialize, Deserialize)]
#[repr(usize)]
pub enum Species {
    /// Cold-adapted migratory deer.
    Reindeer,
    /// Large open-country grazer.
    Bison,
    /// Warm open-country grazer.
    Antelope,
    /// Woodland deer.
    Deer,
    /// Woodland wild pig.
    Boar,
    /// Mountain-adapted goat.
    Ibex,
}
#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct Herd {
    /// Stable herd identifier.
    pub id: u32,
    /// Current spherical cell.
    pub cell: u32,
    /// Species and its biological/harvest parameters.
    pub species: Species,
    /// Living animals in this herd.
    pub animals: u32,
    /// Nutritional condition, 0..1.
    pub condition: f32,
    /// Accumulated travel allowance in kilometers.
    pub travel: f32,
}
impl Species {
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
    pub fn habitat(self, forest: bool, elevation: i32) -> f32 {
        match self {
            Self::Deer | Self::Boar if forest => 14.,
            Self::Ibex if elevation > 1000 => 18.,
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
