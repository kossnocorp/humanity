use serde::{Deserialize, Serialize};
pub const SPECIES_COUNT: usize = 14;

#[derive(Clone, Copy, Debug, PartialEq, Eq, PartialOrd, Ord, Hash, Serialize, Deserialize)]
#[repr(usize)]
pub enum Species {
    /// Cold-adapted migratory grazer.
    Reindeer,
    /// Large open-country grazer.
    Bison,
    /// Warm open-country grazer.
    Antelope,
    /// Woodland/edge browser.
    Deer,
    /// Adaptable woodland omnivore.
    Boar,
    /// Mountain-adapted grazer/browser.
    Ibex,
    /// Fast-breeding open-country lagomorph.
    Hare,
    /// Burrowing, rapidly recruiting colonial lagomorph.
    Rabbit,
    /// Woodland ground bird feeding on seeds and browse.
    Grouse,
    /// Wetland-associated grazing bird.
    WildGoose,
    /// Riparian woody browser with a strong water requirement.
    Beaver,
    /// Open-country equid grazer.
    WildHorse,
    /// Large forest-edge browser.
    Elk,
    /// Slow-recruiting megaherbivore, vulnerable to sustained harvest.
    Mammoth,
}
#[derive(Clone, Copy, Debug)]
pub enum Diet {
    /// Predominantly grass and low vegetation.
    Grazer,
    /// Predominantly woody vegetation and leaves.
    Browser,
    /// Mix of grasses and woody vegetation.
    Mixed,
    /// Fruit/seeds supplemented with vegetation.
    Omnivore,
}
impl Diet {
    /// Preferred resource shares; zero excludes a resource from the diet entirely.
    pub fn weights(self) -> humanity_flora::Forage {
        use humanity_flora::Forage;
        match self {
            Self::Grazer => Forage::new(0.9, 0.1, 0.),
            Self::Browser => Forage::new(0.1, 0.9, 0.),
            Self::Mixed => Forage::new(0.5, 0.5, 0.),
            Self::Omnivore => Forage::new(0.2, 0.2, 0.6),
        }
    }
}

/// Effective population-level assumptions, not literal litter sizes or measured paleo-rates.
pub struct Traits {
    /// Recruits per living animal/year before density, season and condition effects.
    pub recruitment: f32,
    /// Background mortality per animal/year, including unmodelled predation.
    pub mortality: f32,
    /// Animal-food units required each day.
    pub food_need: f32,
    /// Harvested edible adult person-days per animal.
    pub meat: f32,
    /// Preferred temperature in Celsius.
    pub temperature: f32,
    /// Broad thermal tolerance scale; never a hard breeding cutoff.
    pub tolerance: f32,
    /// Approximate carrying density per standard 20 km cell in suitable habitat.
    pub density: f32,
    /// Typical group size above which dispersal is considered.
    pub group_size: u32,
    /// Dispersal attempts per year for sufficiently large healthy populations.
    pub dispersal: f32,
    /// Daily travel accumulation in kilometers.
    pub movement: f32,
    /// Tolerance of human disturbance: zero shy, one tolerant.
    pub human_tolerance: f32,
    /// Benefit of vegetation/terrain cover in evading hunters.
    pub concealment: f32,
    /// Additional capture difficulty after encounter.
    pub difficulty: f32,
    /// Hunter work-days per animal for pursuit/handling.
    pub handling: f32,
    /// Degree of concentration of recruitment in spring/summer, 0..1.
    pub seasonality: f32,
    /// Resource guild.
    pub diet: Diet,
}
impl Species {
    pub const ALL: [Self; SPECIES_COUNT] = [
        Self::Reindeer,
        Self::Bison,
        Self::Antelope,
        Self::Deer,
        Self::Boar,
        Self::Ibex,
        Self::Hare,
        Self::Rabbit,
        Self::Grouse,
        Self::WildGoose,
        Self::Beaver,
        Self::WildHorse,
        Self::Elk,
        Self::Mammoth,
    ];
    pub fn index(self) -> usize {
        self as usize
    }
    pub fn small(self) -> bool {
        matches!(
            self,
            Self::Hare | Self::Rabbit | Self::Grouse | Self::WildGoose | Self::Beaver
        )
    }
    pub fn traits(self) -> Traits {
        // Rates fold in nonbreeding members and ordinary juvenile losses. Small
        // species recover faster; megafauna cannot replace the same harvest pressure.
        let (
            recruitment,
            mortality,
            food_need,
            meat,
            temperature,
            tolerance,
            density,
            group_size,
            dispersal,
            movement,
            human_tolerance,
            concealment,
            difficulty,
            handling,
            seasonality,
            diet,
        ) = match self {
            Self::Reindeer => (
                0.32,
                0.12,
                0.9,
                45.,
                2.,
                22.,
                220.,
                45,
                1.2,
                18.,
                0.25,
                0.3,
                0.08,
                2.,
                0.8,
                Diet::Mixed,
            ),
            Self::Bison => (
                0.26,
                0.10,
                1.8,
                100.,
                12.,
                24.,
                140.,
                30,
                0.8,
                14.,
                0.25,
                0.2,
                0.18,
                5.,
                0.8,
                Diet::Grazer,
            ),
            Self::Antelope => (
                0.42,
                0.16,
                0.7,
                35.,
                25.,
                20.,
                260.,
                25,
                1.4,
                18.,
                0.35,
                0.35,
                0.08,
                1.5,
                0.6,
                Diet::Grazer,
            ),
            Self::Deer => (
                0.42,
                0.14,
                0.8,
                40.,
                12.,
                25.,
                180.,
                12,
                1.3,
                10.,
                0.65,
                0.8,
                0.08,
                2.,
                0.8,
                Diet::Browser,
            ),
            Self::Boar => (
                0.95,
                0.28,
                0.65,
                30.,
                18.,
                24.,
                140.,
                10,
                1.8,
                8.,
                0.85,
                0.8,
                0.12,
                1.5,
                0.4,
                Diet::Omnivore,
            ),
            Self::Ibex => (
                0.30,
                0.12,
                0.55,
                25.,
                6.,
                23.,
                110.,
                12,
                1.0,
                8.,
                0.3,
                0.7,
                0.15,
                2.,
                0.8,
                Diet::Mixed,
            ),
            Self::Hare => (
                2.0,
                0.85,
                0.035,
                1.5,
                10.,
                28.,
                500.,
                25,
                3.0,
                4.,
                0.75,
                0.85,
                0.1,
                0.18,
                0.65,
                Diet::Grazer,
            ),
            Self::Rabbit => (
                3.0,
                1.15,
                0.03,
                1.2,
                16.,
                21.,
                700.,
                35,
                3.0,
                3.,
                0.9,
                0.95,
                0.08,
                0.15,
                0.5,
                Diet::Grazer,
            ),
            Self::Grouse => (
                1.8,
                0.8,
                0.025,
                0.8,
                10.,
                24.,
                400.,
                20,
                2.5,
                5.,
                0.65,
                0.85,
                0.1,
                0.12,
                0.9,
                Diet::Omnivore,
            ),
            Self::WildGoose => (
                0.8,
                0.3,
                0.06,
                2.5,
                10.,
                26.,
                250.,
                30,
                2.,
                20.,
                0.45,
                0.45,
                0.12,
                0.3,
                0.9,
                Diet::Grazer,
            ),
            Self::Beaver => (
                0.5,
                0.18,
                0.12,
                5.,
                10.,
                26.,
                60.,
                8,
                1.2,
                3.,
                0.65,
                0.9,
                0.12,
                0.7,
                0.8,
                Diet::Browser,
            ),
            Self::WildHorse => (
                0.25,
                0.10,
                1.2,
                65.,
                12.,
                26.,
                160.,
                20,
                0.9,
                16.,
                0.2,
                0.2,
                0.16,
                3.5,
                0.8,
                Diet::Grazer,
            ),
            Self::Elk => (
                0.28,
                0.12,
                1.4,
                80.,
                6.,
                25.,
                100.,
                15,
                0.9,
                12.,
                0.4,
                0.7,
                0.15,
                4.,
                0.8,
                Diet::Browser,
            ),
            Self::Mammoth => (
                0.075,
                0.045,
                7.,
                600.,
                2.,
                22.,
                12.,
                4,
                0.35,
                12.,
                0.1,
                0.1,
                0.35,
                25.,
                0.2,
                Diet::Mixed,
            ),
        };
        Traits {
            recruitment,
            mortality,
            food_need,
            meat,
            temperature,
            tolerance,
            density,
            group_size,
            dispersal,
            movement,
            human_tolerance,
            concealment,
            difficulty,
            handling,
            seasonality,
            diet,
        }
    }
    pub fn meat(self) -> f32 {
        self.traits().meat
    }
    pub fn ideal_temperature(self) -> f32 {
        self.traits().temperature
    }
    pub fn difficulty(self) -> f32 {
        self.traits().difficulty
    }
    pub fn habitat(self, forest: bool, elevation: i32) -> f32 {
        match self {
            Self::Deer | Self::Boar | Self::Elk | Self::Grouse if forest => 14.,
            Self::Ibex if elevation > 1000 => 18.,
            Self::Bison | Self::Antelope | Self::WildHorse | Self::Mammoth if forest => -12.,
            _ => 0.,
        }
    }
}
