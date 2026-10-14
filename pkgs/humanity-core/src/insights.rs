use crate::*;
use std::collections::VecDeque;

/// Daily samples for a full century, plus the starting cumulative-count baseline.
pub fn history_capacity(year_days: u32) -> usize {
    year_days as usize * 100 + 1
}
pub const DEATH_RECORDS: usize = 128;

#[derive(Clone, Copy, Debug, Serialize, Deserialize, PartialEq, Eq)]
pub enum DeathCause {
    Nutrition,
    Background,
    Aging,
    Conflict,
}
impl DeathCause {
    pub const ALL: [Self; 4] = [
        Self::Nutrition,
        Self::Background,
        Self::Aging,
        Self::Conflict,
    ];
    pub fn label(self) -> &'static str {
        match self {
            Self::Nutrition => "Nutrition / poor health",
            Self::Background => "Background mortality",
            Self::Aging => "Elder mortality",
            Self::Conflict => "Tribal conflict",
        }
    }
}

#[derive(Clone, Copy, Default, Debug, Serialize, Deserialize, PartialEq, Eq)]
pub struct DeathCounts {
    pub nutrition: u64,
    pub background: u64,
    pub aging: u64,
    pub conflict: u64,
}
impl DeathCounts {
    pub fn get(self, cause: DeathCause) -> u64 {
        match cause {
            DeathCause::Nutrition => self.nutrition,
            DeathCause::Background => self.background,
            DeathCause::Aging => self.aging,
            DeathCause::Conflict => self.conflict,
        }
    }
    pub fn checked_total(self) -> Option<u64> {
        DeathCause::ALL
            .into_iter()
            .try_fold(0_u64, |n, c| n.checked_add(self.get(c)))
    }
    pub fn add(&mut self, cause: DeathCause, count: u32) {
        let field = match cause {
            DeathCause::Nutrition => &mut self.nutrition,
            DeathCause::Background => &mut self.background,
            DeathCause::Aging => &mut self.aging,
            DeathCause::Conflict => &mut self.conflict,
        };
        *field += u64::from(count);
    }
}

#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct PopulationSample {
    pub day: u64,
    pub population: u64,
    pub births: u64,
    pub deaths: DeathCounts,
    pub cohorts: [u64; 3],
    pub mean_health: f32,
    pub low_health: u64,
    pub low_food: u64,
}

#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct DeathRecord {
    pub day: u64,
    pub tribe: u32,
    pub cell: CellId,
    pub cause: DeathCause,
    pub count: u32,
    /// Context at resolution, not a claim that weather directly killed someone.
    pub health: f32,
    pub food_days: f32,
    pub temperature: f32,
    pub snow: f32,
}

#[derive(Clone, Default, Debug, Serialize, Deserialize)]
pub struct PopulationInsights {
    pub since_day: Option<u64>,
    pub deaths: DeathCounts,
    pub history: VecDeque<PopulationSample>,
    pub recent_deaths: VecDeque<DeathRecord>,
}
impl PopulationInsights {
    pub fn death(&mut self, day: u64, tribe: &Tribe, cell: &Cell, cause: DeathCause, count: u32) {
        if count == 0 {
            return;
        }
        self.deaths.add(cause, count);
        if self.recent_deaths.len() == DEATH_RECORDS {
            self.recent_deaths.pop_front();
        }
        self.recent_deaths.push_back(DeathRecord {
            day,
            tribe: tribe.id,
            cell: tribe.cell,
            cause,
            count,
            health: tribe.health,
            food_days: tribe.food / tribe.headcount().max(1) as f32,
            temperature: cell.weather.temperature,
            snow: cell.weather.snow,
        });
    }
}

impl World {
    /// No simulation RNG is consumed by instrumentation.
    pub fn record_population(&mut self) {
        if self.insights.since_day.is_none() {
            self.insights.since_day = Some(self.day);
        }
        let mut sample = PopulationSample {
            day: self.day,
            population: self.population(),
            births: self.totals.births,
            deaths: self.insights.deaths,
            cohorts: [0; 3],
            mean_health: 0.,
            low_health: 0,
            low_food: 0,
        };
        let mut weighted_health = 0_f64;
        for t in &self.tribes {
            let n = u64::from(t.headcount());
            weighted_health += f64::from(t.health) * n as f64;
            if t.health < 0.5 {
                sample.low_health += n;
            }
            if t.food < t.headcount() as f32 {
                sample.low_food += n;
            }
            for (i, total) in sample.cohorts.iter_mut().enumerate() {
                *total += u64::from(t.people.female[i]) + u64::from(t.people.male[i]);
            }
        }
        sample.mean_health = (weighted_health / sample.population.max(1) as f64) as f32;
        if self
            .insights
            .history
            .back()
            .is_some_and(|s| s.day == self.day)
        {
            self.insights.history.pop_back();
        }
        while self.insights.history.len() >= history_capacity(self.config.year_days()) {
            self.insights.history.pop_front();
        }
        self.insights.history.push_back(sample);
    }
}
