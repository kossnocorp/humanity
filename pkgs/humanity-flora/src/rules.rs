/// Broad botanical communities; individual plant species are not simulated yet.
#[derive(Clone, Copy, Debug)]
pub enum Community {
    /// Predominantly herbaceous open vegetation.
    Grassland,
    /// Grassland with scattered woody vegetation.
    Savanna,
    /// Closed temperate woodland with limited understory pasture.
    Forest,
    /// Dense warm woodland with especially little accessible grass.
    Rainforest,
    /// Cold woodland and seasonal understory.
    Taiga,
    /// Low cold-adapted vegetation with few woody plants.
    Tundra,
    /// Sparse, water-limited vegetation.
    Desert,
    /// Sparse mountain vegetation.
    Alpine,
    /// Ice or other nonvegetated surface.
    Bare,
    /// Aquatic production belongs to the aquatic model.
    Water,
}
impl Community {
    /// Potential grass and woody cover fractions, before stress and climate.
    pub fn cover(self) -> (f32, f32) {
        match self {
            Self::Grassland => (0.75, 0.05),
            Self::Savanna => (0.65, 0.2),
            Self::Forest => (0.08, 0.8),
            Self::Rainforest => (0.04, 0.95),
            Self::Taiga => (0.1, 0.6),
            Self::Tundra => (0.25, 0.08),
            Self::Desert => (0.015, 0.015),
            Self::Alpine => (0.15, 0.04),
            Self::Bare | Self::Water => (0., 0.),
        }
    }
}
pub struct FloraRules {
    /// Grass/browse intrinsic logistic rates and fruit's replenishment rate.
    /// Logistic r*K/4 matches the former maximum renewal scale for live forage.
    pub growth_rates: [f32; 3],
    /// Fraction of maximum sustainable production available to modeled fauna.
    /// The remainder represents plant maintenance, inaccessible biomass and other consumers.
    pub wildlife_share: f32,
    /// Standing food must also cover this many days; avoids growth/colonization on empty patches.
    pub reserve_days: f32,
    /// Biomass fraction below which chronic defoliation reduces plant vigor.
    pub depletion_threshold: f32,
    /// Maximum fractional daily vigor loss under sustained depletion.
    pub stress_rate: f32,
    /// Daily vigor recovery when vegetation has recovered enough biomass.
    pub recovery_rate: f32,
    /// Remaining below-ground root/seed reserve, not immediately edible forage.
    pub root_reserve: f32,
}
pub const FLORA: FloraRules = FloraRules {
    growth_rates: [0.14, 0.048, 0.045],
    wildlife_share: 0.5,
    reserve_days: 90.,
    depletion_threshold: 0.2,
    stress_rate: 0.006,
    recovery_rate: 0.0008,
    root_reserve: 0.02,
};
