# Individual-human implementation checks (2026-10-15)

The first v8 implementation extracts humans, bands, skills and nutrition into
dedicated crates. It changes population representation and work/learning mechanics;
these checks are not a calibration to prehistoric death percentages.

## Century run

Fresh seed 42, resolution 28, 20 km cells, temperature 28, 90-day seasons,
6,000 initial people, 36,000 simulated days, unlimited analytics:

- 7,658 living individual records in 209 bands after 100 years.
- 22,211 births; 20,553 deaths; population and cause counters reconcile exactly.
- Lifetime deaths: 1,065 nutrition/poor-health, 10,719 background, 3,828 elder,
  4,941 conflict. Years 90–100 had 196 nutrition/poor-health deaths, out of 2,324.
- 574,124 retained personal-history entries; 42.6 MB JSON snapshot.
- Approximately 77.8 seconds for the headless run and final save on the development
  machine using the normal optimized development profile.
- Every living person has a unique ID and valid chronological birth date. Births
  allocate fresh IDs; membership transfers do not duplicate them.

This seed now demonstrates recovery/growth rather than the previous universal
decline. It does **not** establish robust balance across seeds: low direct nutrition
mortality returns in this favorable mature population. Flexible work and lower child
rations improve provisioning. Explicit disease, realistic fertility schedules and
further ecological calibration remain pending; do not manufacture starvation to
match a particular chart share. See `population-balance.md` and `deaths.md`.

Human 16379 provides an inspectable example: a 48-year-old woman in band 245,
with 634 observed personal events. Her tracking proficiency reflects 452.9
practiced days plus instruction and stories. Her current role can change without
resetting those skills. The viewer shows role, age, skill levels, practice and social
learning separately; CLI `--inspect SAVE --human 16379` returns the full record.

## Scale smoke test

Fresh seed 42, same planet, 100,000 initial people, 30 days, analytics off:

- 99,605 living individuals, with unique IDs and personal histories retained.
- 96.7 MB JSON snapshot; no optional chart samples, world journal or death-context
  records retained.
- Approximately 1.39 seconds for simulation and save (1.43 seconds whole process).
- `/usr/bin/time -l` reported 131,678,208 bytes maximum resident set size, about
  126 MiB. This is a short-run measurement, not a projection for centuries of
  biographies, nor a claim of equal graphical throughput.

JSON remains appropriate for experimentation; conversion to binary is deferred.

## Correctness and inspection

- A 60-day uninterrupted continuation of the century save was byte-identical to
  30 days + save/load + 30 days, including individuals, biographies and RNG state.
- Tests cover actual 15th/50th birthdays, unchanged skills through aging, role
  reassignment, instruction time charged to both people, teacher-relative ceilings,
  practice beyond teaching ceilings, and weaker activity-derived story learning.
- Fission conserves actual IDs, skills, birth dates, sex/age counts and shared goods.
- Analytics off versus unlimited produces identical individual/world gameplay
  state. Capped windows retain the subtraction baseline; restarting analytics
  does not reconstruct missing history. Corrupt personal skills are rejected.
- Viewer rendering and individual CLI inspection were exercised on the century save.

All generated snapshots/screenshots for these checks are in the harness temporary
directory, not replacement saves for an existing user world. Fresh v8 saves are
required; no migration path is provided.
