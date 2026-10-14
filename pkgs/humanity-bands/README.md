# humanity-bands

Owns residential groups, shared food/equipment, membership, language mixtures,
cultural affiliations, current role counts and derived skill/health summaries.
Individuals retain their own authoritative skills and biographies.

- `src/lib.rs`: `Tribe`, `Equipment`, `Language`, summaries and capability queries.
- `src/roles.rs`: assignments driven by plant, prey and aquatic opportunities,
  personal skills, existing assignments' diminishing returns, care and craft needs.
- `src/learning.rs`: bounded apprenticeship and stories arising from actual work.
- `src/membership.rs`: fission transfers existing people and apportions provisions;
  IDs, birth dates and skills survive. Fusion/recruitment remain future work.

Each worker gets one current role. Ten percent of working time supports craft;
the remainder supports the main assignment. Relocation reduces available time.
One apprentice per role/day receives a lesson from a stronger worker; both lose
the lesson's working time. Story listening is a small off-work social-learning
channel, not credited as practiced work. No one learns merely because time passes.

`Knowledge` is a cached mean of living members, not an independent skill pool.
Hunt/fishing/gathering outcomes use participating workers' proficiency. Band means
rise through members' practice and social learning, and change when members leave
or die. Elder expertise remains in personal records and can contribute to work
and teaching. Cultural strategy remains owned by `humanity-cultures`.
