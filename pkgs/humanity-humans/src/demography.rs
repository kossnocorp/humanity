//! Chronological aging, individual births and mortality. No population-stage transfer lottery.
use crate::*;

#[derive(Default)]
pub struct VitalChanges {
    /// New living newborn records created today.
    pub births: u32,
    /// Deaths from the residual non-nutritional hazard.
    pub background: u32,
    /// Deaths from personal nutrition/poor-health risk.
    pub nutrition: u32,
    /// Deaths from the additional elder hazard.
    pub aging: u32,
}
impl VitalChanges {
    pub fn deaths(&self) -> u32 {
        self.background + self.nutrition + self.aging
    }
}

pub fn advance(
    people: &mut Demography,
    day: u64,
    band: u32,
    next_id: &mut u64,
    rng: &mut impl RandomSource,
) -> VitalChanges {
    let mut result = VitalChanges::default();
    let year = people.year_days;
    let mut newborns = Vec::new();
    for h in &mut people.members {
        let age = h.age(day, year);
        if h.age_days(day).is_multiple_of(u64::from(year)) && matches!(age, 15 | 50 | 75) {
            h.record(day, LifeEventKind::Birthday(age));
        }
        if h.sex == Sex::Female
            && (15..50).contains(&age)
            && rng.unit() < 0.16 * h.health.powi(2) / year as f32
        {
            let id = *next_id;
            *next_id += 1;
            h.record(day, LifeEventKind::ChildBorn(id));
            newborns.push(Human {
                id,
                birth_day: day as i64,
                sex: if rng.unit() < 0.49 {
                    Sex::Female
                } else {
                    Sex::Male
                },
                health: h.health,
                skills: Skills::default(),
                role: Role::Dependent,
                work_fraction: 0.,
                history: vec![
                    LifeEvent {
                        day,
                        kind: LifeEventKind::Born,
                    },
                    LifeEvent {
                        day,
                        kind: LifeEventKind::JoinedBand(band),
                    },
                ],
            });
        }
    }
    result.births = newborns.len() as u32;
    people.members.extend(newborns);
    people.members.retain(|h| {
        let background = 0.018 / year as f32;
        let nutrition = (1. - h.health).powi(3) * 0.025;
        let aging = if h.age(day, year) >= 50 {
            0.08 / year as f32
        } else {
            0.
        };
        let roll = rng.unit();
        if roll < background {
            result.background += 1;
            false
        } else if roll < background + nutrition {
            result.nutrition += 1;
            false
        } else if roll < background + nutrition + aging {
            result.aging += 1;
            false
        } else {
            true
        }
    });
    people.refresh(day);
    result
}

#[cfg(test)]
mod tests {
    use super::*;
    struct Quiet;
    impl RandomSource for Quiet {
        fn index(&mut self, _: usize) -> usize {
            0
        }
        fn unit(&mut self) -> f32 {
            0.999
        }
    }
    #[test]
    fn ages_cross_boundaries_only_on_real_birthdays_preserving_personal_skills() {
        let mut next = 0;
        let mut people = Demography::from_counts([1, 0, 0], [0; 3], 360, &mut next);
        people.members[0].skills.levels[Skill::Tracking as usize] = 0.3;
        let id = people.members[0].id;
        advance(&mut people, 15 * 360 - 1, 0, &mut next, &mut Quiet);
        assert_eq!(people.female, [1, 0, 0]);
        advance(&mut people, 15 * 360, 0, &mut next, &mut Quiet);
        assert_eq!(people.female, [0, 1, 0]);
        advance(&mut people, 50 * 360 - 1, 0, &mut next, &mut Quiet);
        assert_eq!(people.female, [0, 1, 0]);
        advance(&mut people, 50 * 360, 0, &mut next, &mut Quiet);
        assert_eq!(people.female, [0, 0, 1]);
        assert_eq!(people.members[0].id, id);
        assert_eq!(people.members[0].skills.level(Skill::Tracking), 0.3);
        assert!(people.members[0].can_work(50 * 360, 360));
    }

    #[test]
    fn birth_eligibility_uses_actual_age_and_newborns_get_new_ids_without_skills() {
        struct Rolls(std::vec::IntoIter<f32>);
        impl RandomSource for Rolls {
            fn index(&mut self, _: usize) -> usize {
                0
            }
            fn unit(&mut self) -> f32 {
                self.0.next().unwrap_or(0.999)
            }
        }
        for age in [14, 15, 49, 50] {
            let mut next = 0;
            let mut people = Demography::from_counts([1, 0, 0], [0; 3], 360, &mut next);
            people.members[0].birth_day = -i64::from(age * 360);
            people.members[0].skills.levels = [0.9; 13];
            let result = advance(
                &mut people,
                0,
                0,
                &mut next,
                &mut Rolls(vec![0., 0.8].into_iter()),
            );
            assert_eq!(result.births, u32::from((15..50).contains(&age)));
            if result.births > 0 {
                let newborn = people.members.iter().find(|h| h.id == 1).unwrap();
                assert_eq!(newborn.birth_day, 0);
                assert_eq!(newborn.skills.levels, [0.; 13]);
            }
        }
    }
}
