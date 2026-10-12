//! A remembered place has a function and an observed occupation history.
use crate::{CellId, SOCIAL};
use serde::{Deserialize, Serialize};

#[derive(Clone, Copy, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub enum SiteKind {
    /// Fishing-oriented residential/procurement location; not a claim of continuous residence.
    FishingSettlement,
    /// Residential place whose seasonal or long-stay use is determined by observations.
    ResidentialCamp,
}

impl SiteKind {
    pub fn label(self) -> &'static str {
        match self {
            Self::FishingSettlement => "Fishing location",
            Self::ResidentialCamp => "Residential camp",
        }
    }
    pub fn decay_years(self) -> f32 {
        match self {
            Self::FishingSettlement => SOCIAL.fishing_site_decay_years,
            Self::ResidentialCamp => SOCIAL.residential_site_decay_years,
        }
    }
}

#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct SiteSeason {
    /// Absolute global season number; local season is stored separately for hemisphere shifts.
    pub period: u64,
    pub season: u32,
    pub occupied_days: u32,
    pub person_days: u64,
}

#[derive(Clone, Debug, Default, Serialize, Deserialize)]
pub struct SiteOccupation {
    /// First day on which this place's occupation is tracked.
    pub since: u64,
    pub occupied_days: u64,
    pub episodes: u64,
    /// Consecutive days occupied by any associated band, not one individual household.
    pub continuous_days: u32,
    pub longest_stay: u32,
    pub present_people: u64,
    /// At most eight absolute seasons, including unoccupied observed seasons.
    pub seasons: Vec<SiteSeason>,
}

#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct CulturalSite {
    pub id: u64,
    pub name: String,
    pub cell: CellId,
    pub culture: u32,
    pub founder_band: u32,
    pub kind: SiteKind,
    /// Known local-season associations; these need not imply measured occupation duration.
    pub known_seasons: u8,
    pub founded: u64,
    pub last_visited: u64,
    /// Usability / maintained knowledge of the place, not a count of buildings.
    pub condition: f32,
    pub last_maintained: Option<u64>,
    pub occupation: SiteOccupation,
}
impl CulturalSite {
    pub fn new(
        id: u64,
        cell: CellId,
        culture: u32,
        founder_band: u32,
        kind: SiteKind,
        day: u64,
        season: u32,
    ) -> Self {
        Self {
            id,
            name: format!(
                "{} {}",
                if kind == SiteKind::FishingSettlement {
                    "Fish landing"
                } else {
                    "Camp"
                },
                id + 1
            ),
            cell,
            culture,
            founder_band,
            kind,
            known_seasons: 1 << season,
            founded: day,
            last_visited: day,
            condition: 1.,
            last_maintained: Some(day),
            occupation: SiteOccupation {
                since: day,
                ..SiteOccupation::default()
            },
        }
    }

    pub fn seasons_label(&self) -> String {
        ["spring", "summer", "autumn", "winter"]
            .into_iter()
            .enumerate()
            .filter(|(i, _)| self.known_seasons & (1 << i) != 0)
            .map(|(_, s)| s)
            .collect::<Vec<_>>()
            .join(" / ")
    }

    /// Descriptions of observed use, independent of cultural strategy or site function.
    pub fn pattern(&self, season_days: u32) -> &'static str {
        let o = &self.occupation;
        if o.seasons.is_empty() {
            return "Not yet observed";
        }
        if o.continuous_days >= season_days * 4 {
            return "Year-round residence (observed)";
        }
        if o.continuous_days >= season_days * 2 {
            return "Long-stay base";
        }
        let mut days = [0_u32; 4];
        for s in &o.seasons {
            days[s.season as usize] += s.occupied_days;
        }
        let seasons = days
            .into_iter()
            .filter(|&n| n >= SOCIAL.site_season_use_days.min(season_days).max(1))
            .count();
        match seasons {
            0 => "Occasional / newly observed",
            1 => "Seasonal use",
            _ => "Multi-season use",
        }
    }

    /// Called exactly once per simulated day, including days without visitors.
    pub fn observe(
        &mut self,
        day: u64,
        season_days: u32,
        season: u32,
        people: u64,
        maintained: bool,
    ) {
        self.condition =
            (self.condition - 1. / (season_days as f32 * 4. * self.kind.decay_years())).max(0.);
        if maintained {
            self.condition = (self.condition + SOCIAL.site_maintenance_repair).min(1.);
            self.last_maintained = Some(day);
        }
        let o = &mut self.occupation;
        let period = day / u64::from(season_days);
        o.seasons.retain(|s| s.period + 8 > period);
        if o.seasons.last().is_none_or(|s| s.period != period) {
            o.seasons.push(SiteSeason {
                period,
                season,
                occupied_days: 0,
                person_days: 0,
            });
        }
        if people > 0 {
            if o.present_people == 0 {
                o.episodes += 1;
            }
            o.continuous_days += 1;
            o.longest_stay = o.longest_stay.max(o.continuous_days);
            o.occupied_days += 1;
            self.last_visited = day;
            let s = o.seasons.last_mut().unwrap();
            s.occupied_days += 1;
            s.person_days += people;
            if s.occupied_days >= SOCIAL.site_season_use_days.min(season_days).max(1) {
                self.known_seasons |= 1 << season;
            }
        } else {
            o.continuous_days = 0;
        }
        o.present_people = people;
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn brief_visits_do_not_imply_permanence_and_unmaintained_occupation_still_decays() {
        let mut site = CulturalSite::new(0, 0, 0, 0, SiteKind::ResidentialCamp, 1, 0);
        for day in 1..=360 {
            let season = (day / 90 % 4) as u32;
            let present = day % 90 < 12;
            site.observe(day, 90, season, if present { 20 } else { 0 }, false);
        }
        assert_eq!(site.known_seasons, 15);
        assert_eq!(site.pattern(90), "Multi-season use");
        assert!(site.occupation.longest_stay <= 12);
        assert!(site.occupation.episodes >= 4);
        assert!(site.condition < 0.7);
        let before = site.condition;
        site.observe(361, 90, 0, 20, false);
        assert!(site.condition < before);
        site.observe(362, 90, 0, 20, true);
        assert!(site.condition > before);
    }
}
