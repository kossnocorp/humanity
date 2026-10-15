use super::*;

impl Tribe {
    /// Children/dependants hear accounts of real craft work, without earning its practice time.
    pub fn craft_stories(&mut self, skill: Skill, work: f32) {
        let source = self
            .people
            .members
            .iter()
            .filter(|h| h.role != Role::Dependent)
            .map(|h| h.skills.level(skill))
            .reduce(f32::max);
        if let Some(source) = source {
            for h in &mut self.people.members {
                if h.role == Role::Dependent {
                    h.skills.learn(skill, source, work, true);
                }
            }
        }
    }
    /// Demand-driven apprenticeship: only people currently assigned this work learn its skill.
    pub fn teach_workers(&mut self, day: u64) {
        for role in [
            Role::Hunter,
            Role::Gatherer,
            Role::Fisher,
            Role::Healer,
            Role::Guide,
        ] {
            let skill = role.skill().unwrap();
            let teacher = self
                .people
                .members
                .iter()
                .filter(|h| h.role == role)
                .max_by(|a, b| a.skills.level(skill).total_cmp(&b.skills.level(skill)))
                .map(|h| (h.id, h.skills.level(skill)));
            let Some((id, level)) = teacher else {
                continue;
            };
            // One apprentice per teacher per day; rotate deterministically among those needing help.
            let candidates: Vec<_> = self
                .people
                .members
                .iter()
                .enumerate()
                .filter(|(_, h)| {
                    h.role == role
                        && h.id != id
                        && h.skills.level(skill) < level * LEARNING.teacher_ceiling
                })
                .map(|(i, _)| i)
                .collect();
            if candidates.is_empty() {
                continue;
            }
            let i = candidates[day as usize % candidates.len()];
            let learner = &mut self.people.members[i];
            if learner
                .skills
                .learn(skill, level, LEARNING.lesson_time, false)
                > 0.
            {
                learner.work_fraction -= LEARNING.lesson_time;
                if !learner.history.iter().any(|e| matches!(e.kind, LifeEventKind::TaughtBy { teacher, skill: s } if teacher == id && s == skill)) {
                    learner.record(day, LifeEventKind::TaughtBy { teacher: id, skill });
                }
                if let Some(t) = self.people.members.iter_mut().find(|h| h.id == id) {
                    t.work_fraction -= LEARNING.lesson_time;
                }
            }
        }
    }
    /// A story exists only after an actual activity. Participants practice; listeners gain less,
    /// below a fraction of the best participant's skill, and gain no practice experience.
    pub fn activity(&mut self, day: u64, role: Role, skill: Skill, work: f32, success: bool) {
        let storyteller = self
            .people
            .members
            .iter()
            .filter(|h| h.role == role && h.work_fraction > 0.)
            .map(|h| h.skills.level(skill))
            .reduce(f32::max);
        let Some(level) = storyteller else {
            return;
        };
        for h in &mut self.people.members {
            if h.role == role {
                h.practice(day, skill, work * h.work_fraction, success);
            } else {
                h.skills.learn(skill, level, work, true);
            }
        }
        self.knowledge = self.people.knowledge(&self.knowledge.prey);
    }
}
