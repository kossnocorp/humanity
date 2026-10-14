use crate::{FORAGING, encounter};

/// Return fraction after locating food, reaching the patch, and weather constraints.
pub fn return_rate(
    stock: f32,
    efficiency: f32,
    area: f32,
    distance: f32,
    snow: f32,
    rain: f32,
    pressure: f32,
) -> f32 {
    efficiency * encounter(stock, area, FORAGING.aquatic_half_density) * (1. - snow * 0.9)
        / (1. + rain * 0.08)
        / (1. + distance / FORAGING.fishing_trip_km)
        / (1. + pressure)
}
/// Subtract only actual catch; luck is supplied by the simulation's persisted RNG.
pub fn catch(stock: &mut f32, capacity: f32, rate: f32, luck: f32) -> f32 {
    let take = stock.min(capacity * rate * (0.6 + luck * 0.4));
    *stock -= take;
    take
}
