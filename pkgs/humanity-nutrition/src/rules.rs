//! Inspectable ecological assumptions, not fitted prehistoric death percentages.

/// Search and travel constraints on daily acquisition of regional standing stocks.
pub struct ForagingRules {
    /// Edible person-days per standard 20 km cell giving half-speed plant gathering.
    pub plant_half_density: f32,
    /// Edible person-days per standard cell giving half-speed fruit gathering.
    pub fruit_half_density: f32,
    /// Edible person-days per standard cell giving half-speed small-game trapping.
    pub game_half_density: f32,
    /// Aquatic person-days per standard cell giving half-speed fishing/shellfish collection.
    pub aquatic_half_density: f32,
    /// One-way distance at which reaching a fishing patch halves usable working time.
    pub fishing_trip_km: f32,
    /// Harvested person-days per standard cell adding one unit of near-camp
    /// depletion. Remote biomass remains, but requires more search to acquire.
    pub patch_harvest_scale: f32,
    /// Seasons for depletion to fall by a factor of e without harvesting.
    pub patch_recovery_seasons: f32,
    /// Fraction of normal working time on a residential relocation day.
    pub moving_work_fraction: f32,
}

/// Baseline search assumptions; technology improves handling but cannot erase search.
pub const FORAGING: ForagingRules = ForagingRules {
    plant_half_density: 300.,
    fruit_half_density: 200.,
    game_half_density: 100.,
    aquatic_half_density: 300.,
    fishing_trip_km: 30.,
    patch_harvest_scale: 4000.,
    patch_recovery_seasons: 0.5,
    moving_work_fraction: 0.5,
};

/// Smooth encounter probability from stock density, rather than full-speed harvest
/// until the last item. Area is in standard 20 km cells, including spherical cell area.
pub fn encounter(stock: f32, area: f32, half_density: f32) -> f32 {
    stock / (stock + area * half_density).max(f32::MIN_POSITIVE)
}
