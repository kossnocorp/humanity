use crate::{Community, FLORA, Forage, Vegetation};

/// Inputs from geography/weather; flora itself consumes no RNG.
pub struct Environment {
    /// Broad vegetation community.
    pub community: Community,
    /// Soil/biome fertility, 0..1.
    pub fertility: f32,
    /// Current temperature in Celsius.
    pub temperature: f32,
    /// Available soil water, 0..1.
    pub moisture: f32,
    /// Snow-covered fraction, 0..1.
    pub snow: f32,
    /// Area in standard 20 km cell units.
    pub area: f32,
    /// Calendar length; used for season-scale production averaging.
    pub year_days: u32,
}

fn grow(stock: &mut f32, capacity: f32, rate: f32, initialize: bool) {
    if initialize {
        *stock = capacity;
        return;
    }
    if capacity <= 0. {
        *stock *= 1. - rate;
        return;
    }
    if *stock > capacity {
        *stock -= (*stock - capacity) * rate;
    } else {
        let roots = capacity * FLORA.root_reserve;
        *stock = (*stock + rate * stock.max(roots) * (1. - *stock / capacity)).min(capacity);
    }
}

pub fn advance(v: &mut Vegetation, environment: Environment, initialize: bool) {
    let Environment {
        community,
        fertility,
        temperature,
        moisture,
        snow,
        area,
        year_days,
    } = environment;
    let (grass, woody) = community.cover();
    if grass + woody == 0. {
        *v = Vegetation::default();
        return;
    }
    let old = [v.grazing, v.browse, v.fruit];
    for ((vigor, stock), capacity) in v.vigor.iter_mut().zip(old).zip(v.capacity.values()).take(2) {
        if initialize {
            *vigor = 1.;
        } else if capacity > 0. {
            let fraction = stock / capacity;
            if fraction < FLORA.depletion_threshold {
                *vigor = (*vigor
                    * (1. - FLORA.stress_rate * (1. - fraction / FLORA.depletion_threshold)))
                    .max(FLORA.root_reserve);
            } else if fraction > 0.5 {
                *vigor += (1. - *vigor) * FLORA.recovery_rate;
            }
        }
    }
    // Picking fruit does not kill the parent plant. Reproductive output follows
    // the underlying live grass/woody community rather than the remaining fruit pile.
    v.vigor[2] = (grass * v.vigor[0] + woody * v.vigor[1]) / (grass + woody);
    // No positive productivity floor for frozen or water-starved plants.
    let warmth =
        ((temperature + 5.) / 25.).clamp(0., 1.) * ((50. - temperature) / 15.).clamp(0., 1.);
    let water = ((moisture - 0.02) / 0.65).clamp(0., 1.);
    let growing = warmth * water * (1. - snow).max(0.);
    let base = fertility * area;
    v.grass_cover = grass * v.vigor[0];
    v.woody_cover = woody * v.vigor[1];
    let capacity = Forage::new(
        // Dormancy stops production, not the existence of all standing dry forage.
        // Applying temperature both to biomass capacity and growth erased winter
        // reserves and made slow-breeding grazers structurally nonviable.
        base * v.grass_cover * 14000.,
        base * v.woody_cover * 10000.,
        base * (grass * 0.2 + woody)
            * growing
            * ((temperature - 8.) / 16.).clamp(0., 1.)
            * v.vigor[2]
            * 6000.,
    );
    let mut supply = [0.; 3];
    for (i, (stock, k)) in [&mut v.grazing, &mut v.browse, &mut v.fruit]
        .into_iter()
        .zip(capacity.values())
        .enumerate()
    {
        let rate = FLORA.growth_rates[i] * growing;
        if i == 2 {
            if initialize {
                *stock = k;
            } else {
                *stock = (*stock + (k - *stock) * FLORA.growth_rates[i]).max(0.);
            }
        } else {
            grow(
                stock,
                k,
                if *stock > k {
                    FLORA.growth_rates[i]
                } else {
                    rate
                },
                initialize,
            );
        }
        // r*K/4 is the maximum production of logistic biomass, not the standing stock.
        // All animal species must share this one budget, with a real reserve constraint.
        let productive_flow = rate * k * if i == 2 { 1. } else { 0.25 } * FLORA.wildlife_share;
        let previous = v.wildlife_supply.values()[i];
        let averaged = if initialize {
            productive_flow
        } else {
            previous + (productive_flow - previous) / (year_days as f32 / 4.).max(1.)
        };
        supply[i] = averaged.min(*stock / FLORA.reserve_days);
    }
    v.capacity = capacity;
    v.wildlife_supply = Forage::new(supply[0], supply[1], supply[2]);
    // Regional edible roots and collected craft materials are harvest stocks,
    // not self-replicating live biomass; retain their existing turnover model.
    let food_warmth =
        ((temperature + 8.) / 25.).clamp(0.02, 1.) * ((55. - temperature) / 15.).clamp(0.05, 1.);
    let food_growing = food_warmth * (0.25 + moisture * 0.75) * (1. - snow * 0.9);
    for (stock, target, rate) in [
        (&mut v.plants, base * (0.15 + food_growing) * 10000., 0.025),
        (&mut v.wood, area * woody * v.vigor[1] * 2500., 0.002),
        (&mut v.fiber, base * (grass + woody * 0.5) * 1000., 0.02),
    ] {
        *stock = if initialize {
            target
        } else {
            (*stock + (target - *stock) * rate).max(0.)
        };
    }
}
