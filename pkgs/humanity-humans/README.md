# humanity-humans

Owns individual living humans (`Human`), stable IDs, signed birth dates, sex,
personal health, independent skills, roles and observed biographies. `Demography`
stores authoritative member records plus derived sex/life-stage display counts.

- `src/lib.rs`: personal state, age queries and population summaries.
- `src/demography.rs`: births, chronological age boundaries and individual mortality.
- Depends on skills and serde, **not** the world, bands or renderer.

A person's age is `(world_day - birth_day) / year_days`; age groups are summaries,
not randomly advancing stages. Initial residents can have negative birth dates.
Initial skill templates are varied between individuals. Newborns start untrained.
Births allocate monotonic global IDs; fission never recreates people.

Living histories include observed birthdays, children born, band transfers, role
changes, skill milestones and useful teachers. Histories begin at generation/birth;
we do not fabricate events before observation. Dead member records are currently
removed, but surviving biographies retain referenced IDs. Disabling analytics
does not remove living people or their histories.

The initial demographic rates remain prototype assumptions: female fertility
15–49 is .16/year times personal health squared; residual mortality .018/year,
additional elder mortality .08/year, and poor-health risk .025×(1-health)^3/day.
These are not a fully calibrated life table or explicit pregnancy/disease model.
Healthy elders can work until 75; detailed age-dependent productivity is future work.
