# Tests

All Rust test bodies live in `tests/unit/`, with one file per topic. Production modules
declare their test module with `#[cfg(test)]` and `#[path]`; this keeps private-state
regressions possible without making implementation details public. There are no inline
test bodies or test-only source files under `src/`.

- `app.rs`: application setup, clock, shortcuts, HUD and retained menu presentation.
- `campaign.rs`, `campaign_military.rs`, `campaign_espionage.rs`: cross-system behavior.
- `economy.rs`, `military.rs`: simulation and combat rules.
- `diplomacy.rs`, `espionage.rs`, `senate.rs`: political rules and evidence.
- `notifications.rs`, `toasts.rs`: delivery, privacy, history and warnings.
- `map.rs`, `map_military.rs`, `terrain.rs`, `water.rs`: geography and rendering invariants.
- `ui_layout.rs`, `military_ui.rs`, `campaign_ui.rs`, `province_ui.rs`,
  `population_ui.rs`: actual panel layout, action dispatch and UI regressions.

`tests/scripts/` contains the existing PowerShell and shell packaging checks.

Run `just test` (all Rust targets) and `just packaging-check`. `just ci` includes
formatting, Clippy, Rust tests, runtime-asset checks, wasm compilation and packaging checks.
