# Tick logo family refresh

## Goal

Bring Tick into the same visual family as mimi while keeping Tick recognizable as the scheduling app.

## Direction

- Keep Tick's original turquoise twin-tail mascot and clock ornaments.
- Match mimi's warm-white square canvas, centered pastel anime portrait, thin circular badge, soft linework, generous negative space, and lowercase handwritten wordmark.
- Use pale mint, turquoise, cream, and restrained muted-gold details so Tick remains distinct from mimi's pink and lavender palette.
- Preserve a 1254 × 1254 master artwork in `src-tauri/icons/app-icon-source.png`, then derive platform icons with the Tauri icon generator.
- Use a separate monochrome clock glyph for the menu bar. The full character artwork is too detailed to remain legible at tray size.
- Crop the circular portrait inside the compact app header so the character remains recognizable at 27 px; keep the complete badge and wordmark everywhere with enough room.
- Use the generated application icon for the README and browser favicon so every public surface shows the same identity.

## Deliverables

- Source artwork and Tauri PNG/ICNS/ICO derivatives.
- A single-color tray glyph with 1× and 2× PNG outputs.
- Updated README artwork, browser favicon, and adaptive macOS tray behavior.

## Verification

- Check the master, 256 px, 64 px, 32 px, and tray outputs visually.
- Run the frontend build, Rust formatting check, Rust tests, and a Tauri bundle build.
- Inspect the built app in the Dock/menu bar and verify the in-app 27 px and 64 px logo placements.
