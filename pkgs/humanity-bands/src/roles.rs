use super::*;

/// Expected food opportunity per worker, calculated by the nutrition/world systems.
pub struct Opportunities {
    /// Relative plant-gathering returns.
    pub gathering: f32,
    /// Relative hunting/trapping returns.
    pub hunting: f32,
    /// Relative aquatic returns; zero means no accessible fishery.
    pub fishing: f32,
}
impl Tribe {
    pub fn assign_roles(&mut self, day: u64, opportunities: Opportunities) {
        let sick = self
            .people
            .members
            .iter()
            .filter(|h| h.health < 0.8)
            .count();
        let workers = self.people.workers() as usize;
        let carers = sick.div_ceil(10).min(workers / 5);
        let mut assigned = [0_usize; 3];
        let yields = [
            opportunities.gathering,
            opportunities.hunting,
            opportunities.fishing,
        ];
        // Stable skill ordering makes scarce experts useful without privileging vector/split order.
        self.people.members.sort_by(|a, b| {
            b.skills
                .levels
                .iter()
                .sum::<f32>()
                .total_cmp(&a.skills.levels.iter().sum::<f32>())
                .then(a.id.cmp(&b.id))
        });
        let mut care = 0;
        let mut guides = 0;
        let need_craft = workers >= 10
            && (self.equipment.stone < self.headcount() as f32 * 0.5
                || self.equipment.wood < self.headcount() as f32 * 0.25
                || (opportunities.fishing > 0.
                    && self.equipment.boat_places < self.headcount() as f32));
        for h in &mut self.people.members {
            h.work_fraction = 1.;
            let role = if !h.can_work(day, self.people.year_days) {
                Role::Dependent
            } else if care < carers {
                care += 1;
                Role::Healer
            } else if need_craft && guides == 0 {
                guides += 1;
                Role::Guide
            } else {
                let skills = [Skill::PlantProcessing, Skill::Tracking, Skill::Fishing];
                let best = (0..3)
                    .max_by(|&a, &b| {
                        let score = |i: usize| {
                            yields[i] * (0.35 + h.skills.level(skills[i]))
                                / (1. + assigned[i] as f32 / (workers.max(1) as f32 * 0.35))
                        };
                        score(a).total_cmp(&score(b)).then(b.cmp(&a))
                    })
                    .unwrap();
                assigned[best] += 1;
                [Role::Gatherer, Role::Hunter, Role::Fisher][best]
            };
            if h.role != role {
                h.role = role;
                h.record(day, LifeEventKind::RoleChanged(role));
            }
        }
        self.refresh(day);
        self.fishing_effort = self.specialists.fishers as f32 / workers.max(1) as f32;
    }
}
