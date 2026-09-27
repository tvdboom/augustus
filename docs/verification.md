# Verification record

Results from the current local working tree on 2026-09-27, after the README audit,
Ay Khanum removal, UI changes and test relocation:

| Check | Result |
| --- | --- |
| `just fmt-check` | Pass |
| `just lint` | Pass; Clippy runs with `-D warnings` |
| `just test` | Pass; 166 tests, zero failures or ignored tests, all default Rust targets |
| `just assets` | Pass; regenerated 20 runtime assets |
| `just assets-check` | Pass; 20 SHA-256-pinned runtime assets verified |
| `just check-wasm` | Pass |
| `just packaging-check` | Pass |
| `just ci` | Pass; includes all checks above except regeneration |
| `just build` | Pass; rebuilt `target/debug/augustus.exe` with the audited changes |
| Test/source structure | 166 test bodies in 20 topic files under `tests/unit`; none in `src` |
| Numbered design audit | Exactly 368 unique rows; referenced source files exist |

The suite includes 25 economy tests, 24 politics tests, 18 military tests, application
integration and command-dispatch regressions, and actual egui layouts at desktop/compact widths.
Both map and campaign tests verify that every one of the ten remaining wonder sites is buildable.

Native visual review of the redesigned panels remains pending after desktop control reported
an Escape stop. The tested layout bounds and transparent animation cells do not certify final
on-screen appearance. Long-term competitive balance also remains unproven. Wonder labor
thresholds remain at the document's values until the user decides on the proposed rescaling.
