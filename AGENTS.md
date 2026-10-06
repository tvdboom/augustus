# Augustus contributor instructions

This repository implements Roman campaigns, local practice, shared audio controls, and Supabase-backed single-player and multiplayer games. Do not restore unrelated space-strategy systems or art. Use `docs/AUGUSTUS-game-design.md` as the game-rule source.

## Structure

- `src/app.rs`: app states, shared resources, and Bevy system registration.
- `src/menu/`: menu screens, forms, wallpapers, and menu audio.
- `src/ui/`: map HUD, panels, notifications, and circular audio controls.
- `src/game/`: local resource simulation and game shortcuts.
- `src/map/map.rs`: historical province map and navigation.
- `src/multiplayer/`: online campaign adapter, delta replay, lobby contracts, and asynchronous Supabase transport.
- `src/platform/config.rs`: public Supabase configuration; never put a service-role key in the client.
- `supabase/schema.sql`: sole database setup source; a complete destructive public-schema reset. No migrations, Edge Functions, or separate backend deployments.
- `assets/`: source wallpapers, audio, and the menu font.
- `assets-runtime/`: generated files; regenerate with `just assets`.

## Working locally

Use `just run` to launch the Augustus executable (`cargo run --package augustus --bin augustus`). Run `just assets` after changing wallpapers or other image assets. KTX-Software 4.x is required by the texture pipeline.

Useful checks are `just check`, `just fmt-check`, `just assets-check`, `just check-wasm`, and `just packaging-check`. Keep package scripts and path-safety checks aligned when changing output paths.

Use `just sql-check` to execute fresh/repeat setup and RPC checks against disposable PostgreSQL. It does not reset the hosted project. Keep single-player and multiplayer snapshot contracts consistent. Preserve caller-only recovery codes, connection leases, atomic preconditioned writes, idempotency, bounded replay, and snapshot retention (48 hours after completion or 30 days since save). Use one outstanding HTTP request per client; never poll full snapshots or echo them after successful saves.

Keep the reference menu's button dimensions, placement, font, form-card structure, footer, and circular audio controls. The Settings page currently contains the retained audio modes; volume is controlled from the shared audio hover control. Add Augustus-specific gameplay settings only when designed. Keep music, volume, mute, and click feedback available from the shared audio controls.
