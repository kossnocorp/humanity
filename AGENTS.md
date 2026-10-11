# Humanity

- Rust / Bevy Civilization-inspired planetary simulation, beginning with mobile hunter-gatherers, before settlements. Procedural planets, not Earth.
- Cargo monorepo: dedicated `humanity-*` crates under `pkgs/`. Keep simulation independent of rendering; prioritize fast incremental builds and large aggregate populations over individual-person entities.
- Geography, climate, seasons, ecology, migration, tribal demographics/languages, and probabilistic events must interact. Seeded headless runs and inspectable saves are first-class.
- Controls: Space pause; -/+ time speed; arrows pan; wheel zoom; drag pan; S save; L load.
- Use simple self-authored SVG assets for the prototype. Maintain runnable instructions in `HUMANS.md`.
- Keep this file slim: record enduring project/process facts supplied by the user, not implementation details derivable from code.
- Before finishing changes, format, run relevant tests and Clippy, and exercise meaningful simulation changes headlessly with saved-state inspection.
