use crate::{FaunaRng, FaunaStats, Herd};

/// Finding prey is separate from capture. Rare/wary animals are difficult to find;
/// visibility falling is not synonymous with animals being absent.
pub fn encounter(group: &Herd, area: f32, cover: f32, distance: f32) -> f32 {
    let traits = group.species.traits();
    let density = group.animals as f32 / area.max(0.001);
    let scale = if group.species.small() { 55. } else { 25. };
    (1. - (-density / scale).exp())
        * (1. - cover * traits.concealment * 0.55)
        * (1. - group.wariness * 0.75)
        / (1. + distance / 20.)
}

impl Herd {
    /// Actual hunting disturbs survivors even when the attempt fails.
    pub fn disturb(&mut self) {
        self.wariness =
            (self.wariness + 0.18 * (1. - self.species.traits().human_tolerance * 0.5)).min(1.);
    }
    /// Conserves animals; the last individual can be harvested, with no protected floor.
    pub fn harvest(&mut self, requested: u32, stats: &mut FaunaStats) -> u32 {
        let killed = requested.min(self.animals);
        self.animals -= killed;
        stats.harvested[self.species.index()] += u64::from(killed);
        self.disturb();
        killed
    }
    /// Small-game trapping spends real work and kills explicit animals, never a regenerating food pool.
    pub fn trap(
        &mut self,
        effort: f32,
        skill: f32,
        encounter: f32,
        rng: &mut impl FaunaRng,
        stats: &mut FaunaStats,
    ) -> (u32, f32) {
        let mean = effort * (0.2 + skill * 0.8) * encounter / self.species.traits().handling;
        let count = rng.expected(mean);
        (self.harvest(count, stats), mean.min(1.))
    }
}
