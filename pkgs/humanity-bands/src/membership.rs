use super::*;
use humanity_humans::Sex;
use std::collections::BTreeSet;

impl Tribe {
    /// Transfer actual identities, preserving every person's skills and biography.
    pub fn split_off(&mut self, id: u32, departing: u32) -> Self {
        let n = self.headcount();
        assert!(departing > 0 && departing < n);
        let mut child = self.clone();
        child.id = id;
        child.parent_band = Some(self.id);
        self.camp_pressure_days = 0;
        child.camp_pressure_days = 0;
        child.people.members.clear();
        let mut selected = BTreeSet::new();
        let mut cumulative = 0_u64;
        let mut assigned = 0_u64;
        for sex in [Sex::Female, Sex::Male] {
            for stage in 0..3 {
                let mut group: Vec<_> = self
                    .people
                    .members
                    .iter()
                    .filter(|h| {
                        h.sex == sex && h.stage(self.people.day, self.people.year_days) == stage
                    })
                    .map(|h| h.id)
                    .collect();
                group.sort_unstable();
                cumulative += group.len() as u64;
                let total = cumulative * u64::from(departing) / u64::from(n);
                selected.extend(group.into_iter().take((total - assigned) as usize));
                assigned = total;
            }
        }
        for mut h in std::mem::take(&mut self.people.members) {
            if selected.contains(&h.id) {
                h.record(self.people.day, LifeEventKind::JoinedBand(id));
                child.people.members.push(h);
            } else {
                self.people.members.push(h);
            }
        }
        let fraction = departing as f32 / n as f32;
        for (source, destination) in [
            (&mut self.food, &mut child.food),
            (&mut self.equipment.wood, &mut child.equipment.wood),
            (&mut self.equipment.fiber, &mut child.equipment.fiber),
            (&mut self.equipment.stone, &mut child.equipment.stone),
            (&mut self.equipment.hides, &mut child.equipment.hides),
            (
                &mut self.equipment.boat_places,
                &mut child.equipment.boat_places,
            ),
        ] {
            *destination = *source * fraction;
            *source -= *destination;
        }
        self.refresh(self.people.day);
        child.refresh(self.people.day);
        child
    }
}
