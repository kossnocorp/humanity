//! Personal proficiency, experience and bounded social learning. No world dependency.
use serde::{Deserialize, Serialize};
pub mod learning;
pub use learning::*;

#[derive(Clone, Copy, Debug, PartialEq, Eq, Serialize, Deserialize)]
#[repr(usize)]
pub enum Skill {
    /// Stone tools and maintenance.
    Knapping,
    /// Cordage, baskets and nets.
    Cordage,
    /// Aquatic food acquisition.
    Fishing,
    /// Building and handling watercraft.
    Watercraft,
    /// Finding and interpreting animal signs.
    Tracking,
    /// Coordinated large-game hunting.
    DriveHunting,
    /// Small-game trapping.
    Trapping,
    /// Finding and preparing edible plants.
    PlantProcessing,
    /// Preserving food for later use.
    Preservation,
    /// Making protective clothing.
    Clothing,
    /// Caring for sick and injured people.
    Healing,
    /// Communicating practical knowledge.
    OralTradition,
    /// Finding safe routes over water.
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

/// A person's independently earned skill state; proficiency is not a population fraction.
#[derive(Clone, Debug, Default, Serialize, Deserialize)]
pub struct Skills {
    /// Proficiency 0..1 for each practical skill (displayable as levels 0..100).
    pub levels: [f32; 13],
    /// Cumulative equivalent working days actually practiced in each skill.
    pub experience: [f32; 13],
    /// Cumulative proficiency gained from instruction, separately inspectable.
    pub taught: [f32; 13],
    /// Cumulative proficiency gained by hearing accounts of real activities.
    pub stories: [f32; 13],
}
impl Skills {
    pub fn level(&self, skill: Skill) -> f32 {
        self.levels[skill as usize]
    }
    pub fn prerequisites(&self, skill: Skill) -> bool {
        prerequisites(skill, |s| self.level(s))
    }
}

/// Derived band summaries, rebuilt from living members; never an independent source of skill growth.
#[derive(Clone, Debug, Default, Serialize, Deserialize)]
pub struct Knowledge {
    /// Mean proficiency among living children, working-age adults and elders.
    pub practitioners: [[f32; 3]; 13],
    /// Shared memory of encounters with each prey species.
    pub prey: [f32; 6],
}
impl Knowledge {
    pub fn level(&self, skill: Skill) -> f32 {
        self.practitioners[skill as usize][1]
    }
    /// Initial generation template, not an ongoing teaching operation.
    pub fn seed(&mut self, skill: Skill, level: f32) {
        self.practitioners[skill as usize] = [level * 0.35, level, (level * 1.2).min(1.)];
    }
    pub fn prerequisites(&self, skill: Skill) -> bool {
        prerequisites(skill, |s| self.level(s))
    }
}
fn prerequisites(skill: Skill, level: impl Fn(Skill) -> f32) -> bool {
    match skill {
        Skill::Watercraft => level(Skill::Cordage) > 0.12 && level(Skill::Knapping) > 0.12,
        Skill::Navigation => level(Skill::Watercraft) > 0.12,
        Skill::Fishing | Skill::Trapping | Skill::Clothing => level(Skill::Cordage) > 0.08,
        Skill::DriveHunting => level(Skill::Tracking) > 0.12,
        _ => true,
    }
}
