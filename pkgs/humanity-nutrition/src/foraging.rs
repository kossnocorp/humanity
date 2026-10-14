use crate::{FORAGING, Resources, encounter};

/// Returns actual gathered food and subtracts it from the corresponding stocks.
pub fn gather(resources: &mut Resources, work: f32, area: f32) -> f32 {
    let fruit_rate = encounter(resources.fruit, area, FORAGING.fruit_half_density);
    let plant_rate = encounter(resources.plants, area, FORAGING.plant_half_density);
    let fruit_work = work * fruit_rate / (fruit_rate + plant_rate).max(f32::MIN_POSITIVE);
    let fruit = resources.fruit.min(fruit_work * fruit_rate);
    let plants = resources.plants.min((work - fruit_work) * plant_rate);
    resources.fruit -= fruit;
    resources.plants -= plants;
    resources.harvested(fruit + plants, area);
    fruit + plants
}
