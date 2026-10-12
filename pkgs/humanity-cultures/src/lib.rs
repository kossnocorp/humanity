//! Inspectable, tunable cultural archetypes and social rules.
//! These are fictional strategies, not historical peoples or a progress ladder.
use serde::{Deserialize, Serialize};

#[derive(Clone, Copy, Debug, Default, PartialEq, Eq, Serialize, Deserialize)]
pub enum CampTradition {
    /// Favors small camps: crowding releases a capped party rather than dividing equally.
    Dispersed,
    /// Tolerates intermediate camp sizes, then divides into roughly equal groups.
    #[default]
    Balanced,
    /// Tolerates larger camps and sends off smaller parties, retaining a larger parent.
    /// Fishing, preservation and plentiful food can raise its crowding threshold.
    RiverBudding,
}

/// Population thresholds and offshoot allocation for one residential custom.
pub struct CampRules {
    /// Human-readable name shown in field notes and inspection summaries.
    pub label: &'static str,
    /// Baseline crowding threshold in people of all ages, before ecological adjustments.
    /// This triggers splitting pressure, rather than preventing births above a hard cap.
    pub normal_limit: u32,
    /// Accumulated crowding days required for an ordinary split; pressure recedes
    /// when population returns within the threshold. Urgent crowding bypasses this wait.
    pub pressure_days: u32,
    /// Fraction of the current population assigned to an offshoot, rounded down,
    /// then capped below and clamped to leave at least one person in each band.
    pub departing_fraction: f32,
    /// Optional additional cap on departing people, calculated as this fraction of
    /// the ecologically adjusted crowding threshold. `None` applies no such cap.
    pub departing_limit_fraction: Option<f32>,
}
pub const CAMPS: [CampRules; 3] = [
    CampRules {
        label: "Small-camp dispersal",
        normal_limit: 40,
        pressure_days: 14,
        departing_fraction: 0.5,
        departing_limit_fraction: Some(0.5),
    },
    CampRules {
        label: "Balanced camp fission",
        normal_limit: 70,
        pressure_days: 28,
        departing_fraction: 0.5,
        departing_limit_fraction: None,
    },
    CampRules {
        label: "River-camp budding",
        normal_limit: 100,
        pressure_days: 45,
        departing_fraction: 1. / 3.,
        departing_limit_fraction: Some(0.5),
    },
];
impl CampTradition {
    pub const ALL: [Self; 3] = [Self::Dispersed, Self::Balanced, Self::RiverBudding];
    pub fn rules(self) -> &'static CampRules {
        &CAMPS[self as usize]
    }
    pub fn label(self) -> &'static str {
        self.rules().label
    }
    pub fn normal_limit(self) -> u32 {
        self.rules().normal_limit
    }
    pub fn pressure_days(self) -> u32 {
        self.rules().pressure_days
    }
}

#[derive(Clone, Copy, Debug, Default, PartialEq, Eq, Serialize, Deserialize)]
pub enum CultureVariant {
    /// Mobile small-camp strategy, weighted toward inland habitats; releases small offshoots.
    DispersedForagers,
    /// Mobile intermediate-size camps that split evenly; the default cultural strategy.
    #[default]
    CooperativeForagers,
    /// Mobile river-oriented bands with fishing/preservation knowledge and budding splits.
    /// This variant does not establish persistent sites.
    RiverNetworks,
    /// Fishing-oriented bands that can establish year-round remembered settlements.
    /// Uses budding splits, so several independent bands can occupy one settlement.
    FishingResidents,
    /// Bands that establish remembered camps tied to local seasons and favor returning
    /// in those seasons; uses balanced fission rather than automatic seasonal band fusion.
    SeasonalRounds,
}
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Residence {
    /// Establishes no persistent sites; normal resource-driven movement remains available.
    Mobile,
    /// Can establish a fishing settlement after sustained, adequately provisioned
    /// residence with fishing knowledge and nearby aquatic stocks; eligible all year.
    Fishing,
    /// Can establish a camp recording the current local season. Its return attraction
    /// applies in that season, with southern seasons shifted by half a year.
    Seasonal,
}

/// Starting strategy, environmental selection weights and initial skill floors.
pub struct VariantRules {
    /// Human-readable strategy name; distinct from the generated culture identity's name.
    pub label: &'static str,
    /// Residential custom assigned to bands when seeded or adopting this variant's culture.
    pub camp: CampTradition,
    /// Determines whether bands can establish fishing settlements, seasonal camps, or neither.
    pub residence: Residence,
    /// Relative selection weight away from rivers and neighboring water cells.
    /// Used for initial seeding and ecological resampling during isolation-driven drift;
    /// zero excludes this variant from that environment's weighted draw.
    pub inland_weight: u32,
    /// Equivalent selection weight on river cells or cells neighboring water.
    /// Weights are normalized across all variants, not interpreted as percentages.
    pub waterside_weight: u32,
    /// Minimum initial adult fishing proficiency (0–1) for newly generated bands.
    /// Raising it seeds associated child/elder proficiency too; later identity changes
    /// and save upgrades do not grant this knowledge automatically.
    pub fishing_floor: f32,
    /// Minimum initial adult preservation proficiency (0–1), applied like `fishing_floor`.
    /// Preservation affects food spoilage/storage and eligibility for larger river camps.
    pub preservation_floor: f32,
}
pub const VARIANTS: [VariantRules; 5] = [
    VariantRules {
        label: "Dispersed foragers",
        camp: CampTradition::Dispersed,
        residence: Residence::Mobile,
        inland_weight: 35,
        waterside_weight: 10,
        fishing_floor: 0.,
        preservation_floor: 0.,
    },
    VariantRules {
        label: "Cooperative foragers",
        camp: CampTradition::Balanced,
        residence: Residence::Mobile,
        inland_weight: 30,
        waterside_weight: 15,
        fishing_floor: 0.,
        preservation_floor: 0.,
    },
    VariantRules {
        label: "River networks",
        camp: CampTradition::RiverBudding,
        residence: Residence::Mobile,
        inland_weight: 5,
        waterside_weight: 25,
        fishing_floor: 0.3,
        preservation_floor: 0.2,
    },
    VariantRules {
        label: "Fishing residents",
        camp: CampTradition::RiverBudding,
        residence: Residence::Fishing,
        inland_weight: 0,
        waterside_weight: 35,
        fishing_floor: 0.5,
        preservation_floor: 0.4,
    },
    VariantRules {
        label: "Seasonal rounds",
        camp: CampTradition::Balanced,
        residence: Residence::Seasonal,
        inland_weight: 30,
        waterside_weight: 15,
        fishing_floor: 0.,
        preservation_floor: 0.3,
    },
];
impl CultureVariant {
    pub const ALL: [Self; 5] = [
        Self::DispersedForagers,
        Self::CooperativeForagers,
        Self::RiverNetworks,
        Self::FishingResidents,
        Self::SeasonalRounds,
    ];
    pub fn rules(self) -> &'static VariantRules {
        &VARIANTS[self as usize]
    }
    pub fn label(self) -> &'static str {
        self.rules().label
    }
}

/// Shared ecological, identity-change and remembered-site parameters.
/// Durations use simulated days/years; food units are the model's person-day equivalents.
pub struct SocialRules {
    /// Raised all-age crowding threshold for river-budding camps meeting every rich-food
    /// condition below. This replaces their normal threshold rather than adding to it.
    pub rich_camp_limit: u32,
    /// Minimum adult fishing proficiency (0–1) for the raised threshold.
    /// Also required to establish a fishing settlement.
    pub rich_fishing: f32,
    /// Minimum adult preservation proficiency (0–1) for the raised threshold.
    pub rich_preservation: f32,
    /// Minimum carried food per person, in food-day units, for the raised threshold.
    pub rich_food_days: f32,
    /// Minimum nearby non-ocean aquatic biomass per person, in food-day units,
    /// for the raised threshold; this check discounts stocks for snow cover.
    pub rich_aquatic_days: f32,
    /// Carried food per person below this value lowers the crowding threshold
    /// and disables remembered-site movement attachment.
    pub scarce_food_days: f32,
    /// Band health (0–1) below this value has the same effects as scarce food.
    pub scarce_health: f32,
    /// Multiplier applied once to the crowding threshold when food or health is scarce;
    /// the resulting all-age population threshold is rounded down.
    pub scarce_limit_fraction: f32,
    /// Crowding-pressure days removed per day at or below the current threshold,
    /// saturating at zero; allows brief crowding interruptions without a full reset.
    pub pressure_recovery: u32,
    /// Population strictly above this multiple of the current threshold splits
    /// immediately. Oversized parents/offshoots can be revisited within the same day.
    pub urgent_multiple: u32,
    /// Years without friendly same-culture band contact before an isolated band can
    /// form a daughter identity. Requires other surviving members of its old culture;
    /// same-culture contact and band fission reset isolation.
    pub isolation_years: u32,
    /// Distinct friendly-contact days with one foreign culture needed to form/adopt
    /// a two-parent mixed identity. Multiple encounters per day count once;
    /// a different counted partner culture restarts accumulation.
    pub mixing_contact_days: u32,
    /// After strictly more than this many years without foreign friendly contact,
    /// discard the remembered partner culture and accumulated mixing-contact days.
    pub contact_memory_years: u32,
    /// Probability (0–1) that an isolation-driven identity change resamples its variant
    /// from local environmental weights. Resampling can choose the existing variant again.
    pub variant_drift_chance: f32,
    /// Probability (0–1) that a newly created mixed identity takes the contact partner's
    /// variant rather than the initiating band's. Later adopters use that shared choice.
    pub mixing_other_variant_chance: f32,
    /// Baseline initial band size as a fraction of its normal customary threshold,
    /// rounded down. Generation also adds the random spread below and respects
    /// the remaining world population.
    pub initial_size_min_fraction: f32,
    /// Width of the uniform integer size variation, as a fraction of the normal
    /// threshold. The rounded-down width is exclusive at its upper end and at least one.
    pub initial_size_spread_fraction: f32,
    /// Consecutive days in the same cell before an eligible band can establish a site.
    /// Migration resets residence time; food and fishing eligibility are checked at founding.
    pub site_founding_days: u32,
    /// Minimum carried food per person, in food-day units, to establish either site type.
    pub site_food_days: f32,
    /// Minimum nearby non-ocean aquatic biomass per person to establish a fishing site.
    /// Unlike the rich-camp check, this founding check uses stocks without a snow discount.
    pub fishing_site_aquatic_days: f32,
    /// Maximum remembered sites per owning culture before further founding is blocked.
    /// Existing sites persist while their founder band or owning culture survives,
    /// including while unoccupied; lowering this value does not delete existing sites.
    pub max_sites_per_culture: usize,
    /// Additive movement-score bonus for the cell of the nearest eligible remembered
    /// site. Applies only with adequate food/health; both cultural and founder sites qualify.
    pub site_loyalty_score: f32,
    /// Movement-score reward per mean-cell-spacing of distance gained toward the nearest
    /// eligible site. Applies to year-round fishing sites and in-season seasonal camps,
    /// only with adequate food/health; normal travel costs and route constraints still apply.
    pub seasonal_return_score: f32,
}
pub const SOCIAL: SocialRules = SocialRules {
    rich_camp_limit: 150,
    rich_fishing: 0.3,
    rich_preservation: 0.25,
    rich_food_days: 8.,
    rich_aquatic_days: 30.,
    scarce_food_days: 3.,
    scarce_health: 0.65,
    scarce_limit_fraction: 0.75,
    pressure_recovery: 2,
    urgent_multiple: 2,
    isolation_years: 3,
    mixing_contact_days: 60,
    contact_memory_years: 1,
    variant_drift_chance: 0.35,
    mixing_other_variant_chance: 0.5,
    initial_size_min_fraction: 0.5,
    initial_size_spread_fraction: 1. / 3.,
    site_founding_days: 20,
    site_food_days: 3.,
    fishing_site_aquatic_days: 10.,
    max_sites_per_culture: 12,
    site_loyalty_score: 4.,
    seasonal_return_score: 8.,
};
