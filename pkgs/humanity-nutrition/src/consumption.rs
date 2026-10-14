/// Age-dependent ration in adult person-days; infancy includes nursing/provisioning
/// costs in this aggregate food budget rather than creating free infant calories.
pub fn ration(age: u32, temperature: f32, clothing: f32) -> f32 {
    let size = match age {
        0..=1 => 0.65,
        2..=5 => 0.55,
        6..=10 => 0.75,
        11..=14 => 0.9,
        _ => 1.,
    };
    size * 0.9 * (1. + (-temperature).max(0.) * 0.006 * (1. - clothing * 0.7))
}
/// Eat a shared ration and return the fraction of need met.
pub fn consume(food: &mut f32, need: f32, preservation: f32, capacity: f32) -> f32 {
    let eaten = food.min(need);
    *food = ((*food - eaten) * (0.97 + preservation * 0.022)).min(capacity);
    eaten / need.max(f32::MIN_POSITIVE)
}
/// Personal response to actual intake; mortality remains a demographic concern.
pub fn health_after(health: f32, nutrition: f32, care: f32) -> f32 {
    (health
        + if nutrition > 0.95 {
            0.008 + care
        } else {
            -(1. - nutrition) * 0.065
        })
    .clamp(0., 1.)
}
