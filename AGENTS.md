# Augustus contributor instructions

This repository currently implements the menu-first Augustus shell, an offline lobby preview, shared audio controls, and the local historical map. Do not restore unrelated space-strategy systems or art. Use `docs/AUGUSTUS-game-design.md` as the future game-rule source.

## Structure

- `src/app.rs`: app states, shared resources, and Bevy system registration.
- `src/menu/`: menu screens, forms, wallpapers, and menu audio.
- `src/ui/`: map HUD, panels, notifications, and circular audio controls.
- `src/game/`: local resource simulation and game shortcuts.
- `src/map/map.rs`: historical province map and navigation.
- `src/multiplayer/`: lobby preview contract and Supabase integration boundary.
- `src/platform/config.rs`: public Supabase configuration; never put a service-role key in the client.
- `supabase/schema.sql`: initial Augustus lobby/player-card schema.
- `assets/`: source wallpapers, audio, and the menu font.
- `assets-runtime/`: generated files; regenerate with `just assets`.

## Working locally

Use `just run` to launch the Augustus executable (`cargo run --package augustus --bin augustus`). Run `just assets` after changing wallpapers or other image assets. KTX-Software 4.x is required by the texture pipeline.

Useful checks are `just check`, `just fmt-check`, `just assets-check`, `just check-wasm`, and `just packaging-check`. Keep package scripts and path-safety checks aligned when changing output paths.

Keep the reference menu's button dimensions, placement, font, form-card structure, footer, and circular audio controls. The Settings page currently contains the retained audio modes; volume is controlled from the shared audio hover control. Add Augustus-specific gameplay settings only when designed. Keep music, volume, mute, and click feedback available from the shared audio controls.
