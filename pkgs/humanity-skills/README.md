# humanity-skills

Owns the 13 skill definitions, prerequisites, personal `Skills` and derived
`Knowledge` summaries. No dependency on people, bands, world or renderer.

`src/learning.rs` contains the inspectable `LEARNING` rules:

| Rule | Default |
|---|---:|
| Teacher-relative learning ceiling | 80% of teacher proficiency |
| Story-relative learning ceiling | 35% of participant proficiency |
| Practice gain | .002 of remaining proficiency per equivalent working day |
| Unsuccessful practice | 40% of successful gain |
| Lesson gain | .04 of teachable gap per full instruction day |
| Story gain | .002 of story-accessible gap per narrated activity unit |
| Lesson time | .15 working day for both teacher and apprentice |

Display proficiency as level 0–100. `experience` stores actual attempted practice
days; `taught` and `stories` separately record social-learning gains. Practice can
surpass a teacher's ceiling. Teaching never lowers a more capable learner or raises
one beyond the source-relative cap. Prerequisites apply to practice and learning.

No elapsed-time growth: the simulator supplies actual attempted activities. A
failed hunt still teaches its hunters, and its account can teach others less.
Initialization and rare environment-conditioned experiments are separate from
practice; those experiments seed one person's knowledge rather than an entire band.
