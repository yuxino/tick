# DeepSeek Settings Implementation Plan

> **For Claude:** REQUIRED SUB-SKILL: Use executing-plans to implement this plan task-by-task.

**Goal:** Add secure in-app DeepSeek API key configuration and restore a compact macOS toolbar.

**Architecture:** Rust owns all Keychain access and DeepSeek requests. React calls narrow Tauri commands for status, save, test, and delete; no command returns the stored key. The existing environment-variable reader is removed.

**Tech Stack:** Tauri 2, React 19, TypeScript, Ant Design, Rust, keyring 3 with Apple native storage.

---

### Task 1: Add the Keychain credential service

**Files:**
- Modify: `src-tauri/Cargo.toml`
- Modify: `src-tauri/src/ai.rs`
- Modify: `src-tauri/src/lib.rs`

1. Add `keyring` with the macOS native backend.
2. Add serializable configuration status with only `configured` and `maskedHint`.
3. Add commands to read status, save, delete, and test the DeepSeek key.
4. Make generation read only from Keychain.
5. Sanitize all credential errors and test the pure validation/masking helpers.

### Task 2: Add the application settings UI

**Files:**
- Create: `src/components/SettingsModal.tsx`
- Modify: `src/services/launchd.ts`
- Modify: `src/App.tsx`
- Modify: `src/App.css`

1. Add typed Tauri service functions.
2. Add a settings button to the toolbar.
3. Build a focused AI-service modal with password input, status, save, test, and delete.
4. Keep secrets out of component persistence and clear the input after successful save.
5. Replace the promotional header sizing with compact toolbar proportions.

### Task 3: Document and verify

**Files:**
- Modify: `README.md`
- Modify: `SECURITY.md`

1. Document in-app setup and Keychain storage.
2. Run `npm run build`.
3. Run `cargo fmt --check` and `cargo test`.
4. Build the macOS app and visually inspect the toolbar and settings modal.
5. Review the Git diff for accidental secret material.
