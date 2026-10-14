//! Residential groups compose individual humans; social identity is a separate cultural layer.
use humanity_cultures::CampTradition;
use humanity_humans::{Demography, LifeEventKind, Role};
use humanity_skills::{Knowledge, LEARNING, Skill};
use serde::{Deserialize, Serialize};
pub mod learning;
pub mod membership;
pub mod roles;
pub use roles::Opportunities;

#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct Language {
    /// Shared language-family identifier.
    pub family: u32,
    /// Fraction of the band's language mixture.
    pub share: f32,
}
#[derive(Clone, Debug, Default, Serialize, Deserialize)]
pub struct Equipment {
    /// Portable wood units.
    pub wood: f32,
    /// Portable cordage/material units.
    pub fiber: f32,
    /// Usable stone-tool units.
    pub stone: f32,
    /// Portable hide units.
    pub hides: f32,
    /// Watercraft passenger capacity.
    pub boat_places: f32,
}
#[derive(Clone, Debug, Default, Serialize, Deserialize)]
pub struct Specialists {
    /// Members assigned hunting/trapping.
    pub hunters: u32,
    /// Members assigned plant gathering.
    pub gatherers: u32,
    /// Members assigned aquatic food acquisition.
    pub fishers: u32,
    /// Members assigned care.
    pub healers: u32,
    /// Members assigned preparation/craft/route support.
    pub guides: u32,
}
impl Specialists {
    /// Generation placeholder; daily assignments are made from real members and opportunities.
    pub fn from_adults(n: u32) -> Self {
        Self {
            gatherers: n,
            ..Self::default()
        }
    }
}
#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct Tribe {
    /// Stable residential-band identity.
    pub id: u32,
    /// Current spherical cell.
    pub cell: u32,
    /// Living individual members and demographic summaries.
    pub people: Demography,
    /// Derived counts of today's assigned roles.
    pub specialists: Specialists,
    /// Shared language affiliations.
    pub languages: Vec<Language>,
    /// Shared edible provisions in adult person-days.
    pub food: f32,
    /// Derived mean member health.
    pub health: f32,
    /// Accumulated travel capacity in kilometers.
    pub travel: f32,
    /// Derived proficiency of members assigned to hunt.
    pub hunting_skill: f32,
    /// Derived member skill summaries and shared prey memory.
    pub knowledge: Knowledge,
    /// Shared materials and transport equipment.
    pub equipment: Equipment,
    /// Today's fraction of assigned workers fishing, for inspection.
    pub fishing_effort: f32,
    /// Cultural residential custom.
    pub camp_tradition: CampTradition,
    /// Consecutive crowding days.
    pub camp_pressure_days: u32,
    /// Band from which this group split.
    pub parent_band: Option<u32>,
    /// Cultural identity index.
    pub culture: u32,
    /// Time isolated from cultural peers.
    pub isolation_days: u32,
    /// Last friendly contact day.
    pub last_cultural_contact: u64,
    /// Foreign identity recently encountered.
    pub contact_culture: Option<u32>,
    /// Accumulated contact duration.
    pub contact_days: u32,
    /// Last foreign contact day.
    pub last_foreign_contact: u64,
    /// Consecutive days at the current location.
    pub residence_days: u32,
}
impl Tribe {
    pub fn headcount(&self) -> u32 {
        self.people.total()
    }
    pub fn refresh(&mut self, day: u64) {
        self.people.refresh(day);
        self.health = self.people.members.iter().map(|h| h.health).sum::<f32>()
            / self.headcount().max(1) as f32;
        self.knowledge = self.people.knowledge(self.knowledge.prey);
        self.specialists = Specialists::default();
        for h in &self.people.members {
            match h.role {
                Role::Hunter => self.specialists.hunters += 1,
                Role::Gatherer => self.specialists.gatherers += 1,
                Role::Fisher => self.specialists.fishers += 1,
                Role::Healer => self.specialists.healers += 1,
                Role::Guide => self.specialists.guides += 1,
                Role::Dependent => (),
            }
        }
        self.hunting_skill = self.role_skill(Role::Hunter, Skill::Tracking);
    }
    pub fn role_skill(&self, role: Role, skill: Skill) -> f32 {
        let mut n = 0_f32;
        let mut total = 0.;
        for h in &self.people.members {
            if h.role == role {
                n += 1.;
                total += h.skills.level(skill);
            }
        }
        total / n.max(1.)
    }
    pub fn labor(&self, role: Role) -> f32 {
        self.people
            .members
            .iter()
            .filter(|h| h.role == role)
            .map(|h| h.health * h.work_fraction)
            .sum()
    }
    pub fn health_shock(&mut self, factor: f32) {
        for h in &mut self.people.members {
            h.health *= factor;
        }
        self.refresh(self.people.day);
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
