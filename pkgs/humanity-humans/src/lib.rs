//! Individual humans and chronological population state. No world or rendering dependency.
use humanity_skills::{Knowledge, Skill, Skills};
use serde::{Deserialize, Serialize};
pub mod demography;

/// Random selection supplied by the simulation's persisted RNG.
pub trait RandomSource {
    fn index(&mut self, length: usize) -> usize;
    fn unit(&mut self) -> f32;
}

#[derive(Clone, Copy, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub enum Sex {
    /// Female reproductive physiology in the prototype demographic model.
    Female,
    /// Male reproductive physiology in the prototype demographic model.
    Male,
}

#[derive(Clone, Copy, Debug, Default, PartialEq, Eq, Serialize, Deserialize)]
pub enum Role {
    /// Not currently assigned productive work.
    #[default]
    Dependent,
    /// Tracking, trapping and hunting land animals.
    Hunter,
    /// Gathering and processing plants.
    Gatherer,
    /// Fishing and shellfish collection.
    Fisher,
    /// Care for injured or ill members.
    Healer,
    /// Travel, equipment and route preparation.
    Guide,
}
impl Role {
    pub fn skill(self) -> Option<Skill> {
        match self {
            Self::Dependent => None,
            Self::Hunter => Some(Skill::Tracking),
            Self::Gatherer => Some(Skill::PlantProcessing),
            Self::Fisher => Some(Skill::Fishing),
            Self::Healer => Some(Skill::Healing),
            Self::Guide => Some(Skill::Knapping),
        }
    }
}

#[derive(Clone, Debug, Serialize, Deserialize)]
pub enum LifeEventKind {
    /// Born on the recorded day.
    Born,
    /// Already alive when observation began; earlier biography is unknown.
    FirstObserved,
    /// Gave birth to the individual with this stable identity.
    ChildBorn(u64),
    /// Joined a residential group, including founding and fission.
    JoinedBand(u32),
    /// Crossed a chronological life-stage boundary.
    Birthday(u32),
    /// Began a different working assignment.
    RoleChanged(Role),
    /// Reached a new ten-point proficiency milestone.
    SkillMilestone { skill: Skill, level: u8 },
    /// Received useful instruction from this specific person.
    TaughtBy { teacher: u64, skill: Skill },
}
#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct LifeEvent {
    /// Simulation day on which the event was observed.
    pub day: u64,
    /// Personal event, retained independently of optional population analytics.
    pub kind: LifeEventKind,
}

#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct Human {
    /// Globally unique identity; preserved through band transfers.
    pub id: u64,
    /// Day of birth; negative for people born before world generation.
    pub birth_day: i64,
    /// Sex used for demographic rates.
    pub sex: Sex,
    /// Personal health 0..1, affecting work, fertility and mortality.
    pub health: f32,
    /// Independent proficiency and activity-derived experience.
    pub skills: Skills,
    /// Current assignment; no permanent job class.
    pub role: Role,
    /// Today's productive time remaining after instruction, 0..1.
    pub work_fraction: f32,
    /// Observed biography of this living human; not the optional global analytics log.
    pub history: Vec<LifeEvent>,
}
impl Human {
    pub fn age_days(&self, day: u64) -> u64 {
        (day as i64 - self.birth_day).max(0) as u64
    }
    pub fn age(&self, day: u64, year: u32) -> u32 {
        (self.age_days(day) / u64::from(year)) as u32
    }
    pub fn stage(&self, day: u64, year: u32) -> usize {
        match self.age(day, year) {
            0..=14 => 0,
            15..=49 => 1,
            _ => 2,
        }
    }
    pub fn can_work(&self, day: u64, year: u32) -> bool {
        (15..75).contains(&self.age(day, year)) && self.health > 0.15
    }
    pub fn record(&mut self, day: u64, kind: LifeEventKind) {
        self.history.push(LifeEvent { day, kind });
    }
    pub fn practice(&mut self, day: u64, skill: Skill, work: f32, success: bool) {
        let old = (self.skills.level(skill) * 10.).floor() as u8;
        self.skills.practice(skill, work, success);
        let new = (self.skills.level(skill) * 10.).floor() as u8;
        if new > old {
            self.record(
                day,
                LifeEventKind::SkillMilestone {
                    skill,
                    level: new * 10,
                },
            );
        }
    }
}

/// Individual storage with derived display counts; counts never drive independent births/aging.
#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct Demography {
    /// Authoritative living member records.
    pub members: Vec<Human>,
    /// Derived female counts for children/adults/elders.
    pub female: [u32; 3],
    /// Derived male counts for children/adults/elders.
    pub male: [u32; 3],
    /// Calendar length used to interpret birth dates.
    pub year_days: u32,
    /// Day at which summaries were last refreshed.
    pub day: u64,
}
impl Demography {
    pub fn from_counts(
        female: [u32; 3],
        male: [u32; 3],
        year_days: u32,
        next_id: &mut u64,
    ) -> Self {
        let mut result = Self {
            members: Vec::new(),
            female,
            male,
            year_days,
            day: 0,
        };
        for (sex, counts) in [(Sex::Female, female), (Sex::Male, male)] {
            for (stage, count) in counts.into_iter().enumerate() {
                let (minimum, span) = [(0, 15), (15, 35), (50, 20)][stage];
                for i in 0..count {
                    // Stratify birthdays within each stage without consuming gameplay RNG.
                    let age_days = minimum * year_days
                        + (u64::from(i) * u64::from(span * year_days) / u64::from(count.max(1)))
                            as u32;
                    result.members.push(Human {
                        id: *next_id,
                        birth_day: -i64::from(age_days),
                        sex,
                        health: 1.,
                        skills: Skills::default(),
                        role: Role::Dependent,
                        work_fraction: 1.,
                        history: vec![LifeEvent {
                            day: 0,
                            kind: if age_days == 0 {
                                LifeEventKind::Born
                            } else {
                                LifeEventKind::FirstObserved
                            },
                        }],
                    });
                    *next_id += 1;
                }
            }
        }
        result
    }
    pub fn total(&self) -> u32 {
        self.members.len() as u32
    }
    pub fn adults(&self) -> u32 {
        self.female[1] + self.male[1]
    }
    pub fn workers(&self) -> u32 {
        self.members
            .iter()
            .filter(|h| h.can_work(self.day, self.year_days))
            .count() as u32
    }
    pub fn refresh(&mut self, day: u64) {
        self.day = day;
        self.female = [0; 3];
        self.male = [0; 3];
        for h in &self.members {
            let counts = if h.sex == Sex::Female {
                &mut self.female
            } else {
                &mut self.male
            };
            counts[h.stage(day, self.year_days)] += 1;
        }
    }
    pub fn remove(&mut self, count: u32, rng: &mut impl RandomSource) -> u32 {
        let count = count.min(self.total());
        for _ in 0..count {
            let index = rng.index(self.members.len());
            self.members.swap_remove(index);
        }
        self.refresh(self.day);
        count
    }
    /// Initial skill templates are copied into distinct member state, with stable individual variation.
    pub fn seed_skills(&mut self, knowledge: &Knowledge, band: u32) {
        for h in &mut self.members {
            let stage = h.stage(self.day, self.year_days);
            let variation =
                0.8 + ((h.id.wrapping_mul(6364136223846793005) >> 56) as f32 / 255.) * 0.4;
            for skill in Skill::ALL {
                h.skills.levels[skill as usize] =
                    (knowledge.practitioners[skill as usize][stage] * variation).min(1.);
            }
            h.record(self.day, LifeEventKind::JoinedBand(band));
        }
    }
    pub fn knowledge(&self, prey: [f32; 6]) -> Knowledge {
        let mut k = Knowledge {
            prey,
            ..Knowledge::default()
        };
        let mut counts = [0_u32; 3];
        for h in &self.members {
            let stage = h.stage(self.day, self.year_days);
            counts[stage] += 1;
            for skill in Skill::ALL {
                k.practitioners[skill as usize][stage] += h.skills.level(skill);
            }
        }
        for levels in &mut k.practitioners {
            for (level, count) in levels.iter_mut().zip(counts) {
                *level /= count.max(1) as f32;
            }
        }
        k
    }
}
