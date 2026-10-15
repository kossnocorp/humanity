/// Outcome probability from participating hunters' actual skills and conditions.
pub struct Hunt {
    /// Participating hunters' mean tracking proficiency.
    pub tracking: f32,
    /// Participating hunters' cooperative-hunt proficiency.
    pub cooperative: f32,
    /// Effective hunting labor after teaching and relocation.
    pub hunters: f32,
    /// Participating hunters' tool-working proficiency.
    pub tool_skill: f32,
    /// Usable tools per resident.
    pub tools: f32,
    /// Shared familiarity with the prey species.
    pub familiarity: f32,
    /// Species-specific hunting difficulty.
    pub difficulty: f32,
    /// Snow cover at the hunting location.
    pub snow: f32,
    /// Whether the herd is outside the residential cell.
    pub distant: bool,
}
pub fn success_probability(hunt: Hunt) -> f32 {
    let Hunt {
        tracking,
        cooperative,
        hunters,
        tool_skill,
        tools,
        familiarity,
        difficulty,
        snow,
        distant,
    } = hunt;
    (0.12
        + tracking * 0.45
        + hunters * 0.003
        + cooperative * 0.15
        + tool_skill * tools.min(1.) * 0.1
        + familiarity * 0.25
        - difficulty
        - snow * 0.15
        - if distant { 0.15 } else { 0. })
    .clamp(0.05, 0.9)
}
