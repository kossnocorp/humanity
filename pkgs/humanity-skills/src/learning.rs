use crate::{Skill, Skills};

/// Inspectable learning assumptions, not elapsed-time technology advancement.
pub struct LearningRules {
    /// An apprentice can learn at most this fraction of the teacher's current proficiency.
    pub teacher_ceiling: f32,
    /// Stories can convey only this fraction of the participant's proficiency.
    pub story_ceiling: f32,
    /// Fraction of remaining proficiency acquired per full practiced working day.
    pub practice_gain: f32,
    /// Fraction of the teachable gap closed per full day of instruction.
    pub lesson_gain: f32,
    /// Fraction of the story-accessible gap learned per actual narrated activity.
    pub story_gain: f32,
    /// Working-day fraction spent by both teacher and apprentice in a lesson.
    pub lesson_time: f32,
}
pub const LEARNING: LearningRules = LearningRules {
    teacher_ceiling: 0.8,
    story_ceiling: 0.35,
    practice_gain: 0.002,
    lesson_gain: 0.04,
    story_gain: 0.002,
    lesson_time: 0.15,
};

impl Skills {
    /// Attempted work teaches even on failure, but nobody gains practice from another's work.
    pub fn practice(&mut self, skill: Skill, work_days: f32, success: bool) {
        if work_days <= 0. || !self.prerequisites(skill) {
            return;
        }
        let i = skill as usize;
        self.experience[i] += work_days;
        let gain = LEARNING.practice_gain * work_days * if success { 1. } else { 0.4 };
        self.levels[i] = (self.levels[i] + (1. - self.levels[i]) * gain).min(1.);
    }
    /// Returns actual gain. No regression when a learner already exceeds the teacher's cap.
    pub fn learn(&mut self, skill: Skill, teacher: f32, time: f32, story: bool) -> f32 {
        if !self.prerequisites(skill) || time <= 0. {
            return 0.;
        }
        let (ceiling, rate) = if story {
            (LEARNING.story_ceiling, LEARNING.story_gain)
        } else {
            (LEARNING.teacher_ceiling, LEARNING.lesson_gain)
        };
        let i = skill as usize;
        let target = teacher.clamp(0., 1.) * ceiling;
        let gain = ((target - self.levels[i]).max(0.) * rate * time)
            .min((target - self.levels[i]).max(0.));
        self.levels[i] += gain;
        if story {
            self.stories[i] += gain;
        } else {
            self.taught[i] += gain;
        }
        gain
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn weak_teachers_and_stories_cannot_create_experts_but_practice_can() {
        let mut learner = Skills::default();
        for _ in 0..10000 {
            learner.learn(Skill::Tracking, 0.2, 1., false);
        }
        assert!(learner.level(Skill::Tracking) <= 0.2 * LEARNING.teacher_ceiling);
        for _ in 0..2000 {
            learner.practice(Skill::Tracking, 1., true);
        }
        assert!(learner.level(Skill::Tracking) > 0.9);
        let before = learner.level(Skill::Tracking);
        learner.learn(Skill::Tracking, 0.1, 1., false);
        assert_eq!(before, learner.level(Skill::Tracking));
        let mut listener = Skills::default();
        for _ in 0..10000 {
            listener.learn(Skill::Tracking, 1., 1., true);
        }
        assert!(listener.level(Skill::Tracking) <= LEARNING.story_ceiling);
        assert_eq!(listener.experience[Skill::Tracking as usize], 0.);
    }
}
