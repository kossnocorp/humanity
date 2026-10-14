# Humanity

- Rust / Bevy Civilization-inspired planetary simulation, beginning with mobile hunter-gatherers, before settlements. Procedural planets, not Earth.
- The planet is a real 3D sphere in both rendering and simulation topology. Predominantly hexagonal cells; necessary non-hexagonal cells are acceptable.
- Cargo monorepo: dedicated `humanity-*` crates under `pkgs/`. Keep simulation independent of rendering; prioritize fast incremental builds and large aggregate populations over individual-person entities.
- Geography, climate, seasons, ecology, migration, tribal demographics/languages, and probabilistic events must interact. Seeded headless runs and inspectable saves are first-class.
- Bands share evolving cultures; isolation and mixing can form new identities. Remembered camps support multi-season use and decay without visits/upkeep. Keep cultural variants and tuning parameters inspectable in a dedicated crate. Existing-save compatibility is not required.
- Until the user explicitly says otherwise, discard old save formats: no backward compatibility, migrations, or fallback initialization for missing save data. Require fresh saves when the format changes.
- Controls: Space pause; -/+ time speed; arrows/drag rotate the globe; wheel zoom; S save; L load.
- `?` opens live population/mortality charts with persisted diagnostics; Space and -/+ still control time there. `F` follows a moving band, keeping it centered.
- Use simple self-authored SVG assets for the prototype. Maintain runnable instructions in `HUMANS.md`.
- Keep this file slim: record enduring project/process facts supplied by the user, not implementation details derivable from code.
- Before finishing changes, format, run relevant tests and Clippy, and exercise meaningful simulation changes headlessly with saved-state inspection.
- Use the standard Cargo `target/` directory for builds and checks.
