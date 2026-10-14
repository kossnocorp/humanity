//! Hydrological variability belongs to the environment, not cultural variants.
pub struct ClimateRules {
    /// Broad and regional oscillation periods, in simulated years.
    pub periods: [f64; 2],
    /// Broad, regional x, and regional z rainfall amplitudes.
    pub amplitudes: [f32; 3],
    /// Minimum/maximum precipitation and recharge multipliers.
    pub rainfall_bounds: [f32; 2],
    /// Freshwater habitat fraction retained at zero soil moisture.
    pub freshwater_refuge: f32,
}
pub const CLIMATE: ClimateRules = ClimateRules {
    periods: [7.3, 3.7],
    amplitudes: [0.75, 0.55, 0.35],
    rainfall_bounds: [0.08, 1.9],
    freshwater_refuge: 0.1,
};
