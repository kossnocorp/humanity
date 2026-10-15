use super::*;

fn environment(community: Community) -> Environment {
    Environment {
        community,
        fertility: 0.8,
        temperature: 25.,
        moisture: 1.,
        snow: 0.,
        area: 1.,
        year_days: 360,
    }
}

#[test]
fn forests_supply_browse_instead_of_an_extra_full_grassland() {
    let mut grass = Vegetation::default();
    let mut forest = Vegetation::default();
    advance(&mut grass, environment(Community::Grassland), true);
    advance(&mut forest, environment(Community::Forest), true);
    assert!(forest.grazing < grass.grazing * 0.2);
    assert!(forest.wildlife_supply.grazing < grass.wildlife_supply.grazing * 0.2);
    assert!(forest.browse > grass.browse * 10.);
    assert!(forest.wildlife_supply.browse > grass.wildlife_supply.browse * 10.);
}

#[test]
fn depleted_pasture_loses_productivity_and_rest_recovers_it_gradually() {
    let mut rested = Vegetation::default();
    advance(&mut rested, environment(Community::Grassland), true);
    let mut grazed = rested.clone();
    for _ in 0..240 {
        grazed.grazing *= 0.05;
        advance(&mut grazed, environment(Community::Grassland), false);
        advance(&mut rested, environment(Community::Grassland), false);
    }
    assert!(grazed.vigor[0] < rested.vigor[0] * 0.5);
    assert!(grazed.wildlife_supply.grazing < rested.wildlife_supply.grazing * 0.1);
    let damaged = grazed.vigor[0];
    advance(&mut grazed, environment(Community::Grassland), false);
    assert!(
        grazed.vigor[0] < 0.5,
        "rest must not instantly reset vegetation"
    );
    for _ in 0..730 {
        advance(&mut grazed, environment(Community::Grassland), false);
    }
    assert!(grazed.vigor[0] > damaged);
    assert!(grazed.wildlife_supply.grazing > 0.);
}

#[test]
fn bare_or_exhausted_dry_ground_has_no_positive_forage_floor() {
    let mut v = Vegetation::default();
    advance(&mut v, environment(Community::Bare), true);
    assert_eq!(v.wildlife_supply.total(), 0.);
    assert_eq!(v.grazing + v.browse + v.fruit, 0.);
    advance(&mut v, environment(Community::Grassland), true);
    v.grazing = 0.;
    v.browse = 0.;
    v.fruit = 0.;
    let mut dry = environment(Community::Grassland);
    dry.moisture = 0.;
    advance(&mut v, dry, false);
    assert_eq!(v.wildlife_supply.grazing, 0.);
    assert_eq!(v.wildlife_supply.browse, 0.);
    assert_eq!(v.grazing + v.browse, 0.);
}

#[test]
fn winter_dormancy_does_not_instantly_erase_standing_forage() {
    let mut v = Vegetation::default();
    advance(&mut v, environment(Community::Grassland), true);
    let standing = v.grazing;
    let supply = v.wildlife_supply.grazing;
    let mut winter = environment(Community::Grassland);
    winter.temperature = -20.;
    winter.snow = 1.;
    advance(&mut v, winter, false);
    assert_eq!(v.grazing, standing);
    assert!(v.wildlife_supply.grazing < supply);
    assert!(v.wildlife_supply.grazing <= v.grazing / rules::FLORA.reserve_days);
}
