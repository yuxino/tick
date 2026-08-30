# Tick Windows Task Scheduler Support Implementation Plan

**Goal:** Add safe, user-level Windows Task Scheduler support while preserving Tick's macOS LaunchAgent behavior and proving the Windows ARM64 native lifecycle.

**Architecture:** A platform-neutral scheduler command layer dispatches to macOS launchd or Windows Task Scheduler adapters. Windows tasks execute a fixed Tick runner with a validated job ID; the runner launches user programs with process APIs and derived Tick-owned paths, never a shell-composed command.

**Tech Stack:** Tauri 2, Rust, Windows Task Scheduler COM, Windows Job Objects, React/TypeScript, NSIS.

---

### Task 1: Protect scheduler identity and paths

**Files:**
- Modify: `src-tauri/src/scheduler/models.rs`
- Modify: `src-tauri/src/scheduler/paths.rs`
- Modify: `src-tauri/src/scheduler/registry.rs`
- Modify: `src-tauri/src/scheduler/validation.rs`

1. Add failing tests for malformed IDs, forged labels, forged registry paths, invalid environment keys, and platform interval limits.
2. Run `cargo test --manifest-path src-tauri/Cargo.toml --locked` and confirm the new tests fail.
3. Implement strict ID/label validation and derive every managed path from validated identity.
4. Run the focused Rust tests and confirm they pass.

### Task 2: Add a shell-free cross-platform runner

**Files:**
- Create: `src-tauri/src/scheduler/executor.rs`
- Modify: `src-tauri/src/scheduler/plist_writer.rs`
- Modify: `src-tauri/src/main.rs`
- Modify: `src-tauri/src/lib.rs`

1. Add tests for Windows paths, spaces, Unicode, empty arguments, trailing backslashes and metacharacters.
2. Implement materialization and argv construction without a shell.
3. Add `--run-scheduled-job <validated-id>` before Tauri startup.
4. On Windows, create the child suspended, attach it to a kill-on-close Job Object before resuming it, and wait for its exit while appending stdout/stderr.
5. Run the Rust tests and both macOS and Windows target checks.

### Task 3: Implement the Windows Task Scheduler adapter

**Files:**
- Create: `src-tauri/src/scheduler/task_scheduler.rs`
- Modify: `src-tauri/src/scheduler/commands.rs`
- Modify: `src-tauri/src/scheduler/mod.rs`
- Modify: `src-tauri/Cargo.toml`
- Modify: `src-tauri/Cargo.lock`

1. Add pure tests for daily/monthly/yearly/interval XML, XML escaping, safe action fields and ownership markers.
2. Connect to Task Scheduler on an isolated COM thread.
3. Register only exact Tick task names with `InteractiveToken` and `LeastPrivilege`; reject foreign collisions.
4. Implement status, enable, run, stop-disable and stop-delete with ownership checks before every mutation.
5. Keep the macOS launchctl/plist adapter behavior behind target-specific compilation.

### Task 4: Platformize the UI and package

**Files:**
- Rename: `src/types/launchd.ts` to `src/types/scheduler.ts`
- Rename: `src/services/launchd.ts` to `src/services/scheduler.ts`
- Rename: `src/utils/launchd.ts` to `src/utils/scheduler.ts`
- Rename: `src/components/PlistPanel.tsx` to `src/components/DefinitionPanel.tsx`
- Modify: `src/App.tsx`
- Modify: `src/components/AutomationModal.tsx`
- Modify: `src/components/JobFormModal.tsx`
- Modify: `src/utils/paths.ts`
- Modify: `src-tauri/tauri.conf.json`
- Create: `src-tauri/tauri.macos.conf.json`
- Create: `src-tauri/tauri.windows.conf.json`
- Create: `src-tauri/icons/icon.ico`

1. Add scheduler capability metadata for labels, path examples, interpreter and interval bounds.
2. Replace macOS-only user copy and plist field names with capability-driven copy.
3. Move macOS titlebar settings to the macOS override and make Windows NSIS explicitly current-user.
4. Generate and inspect a real multi-size ICO from Tick's existing icon source.
5. Run `npm run build` and verify the final frontend output.

### Task 5: Align documentation and automated builds

**Files:**
- Modify: `README.md`
- Modify: `CONTRIBUTING.md`
- Modify: `SECURITY.md`

1. Describe macOS and Windows behavior, storage, permissions, interval limits and current release boundary without claiming publication.
2. Keep CI unchanged unless a hosted runner can prove the actual Windows target reliably; document the host cross-build separately from native lab evidence.
3. Verify local links, paths, commands and `git diff --check`.

### Task 6: Native Windows ARM64 acceptance and delivery

Status: host-side source, tests and package preparation complete first. Run `307e21ee-ce24-4a2e-96db-8a7da4d872c8` is owned by another project and must not be reused; guest install and interaction remain deferred until a fresh Tick run receives an exclusive native VM slot.

**Files:**
- Create: `docs/validation/windows-11-arm64-2026-08-30.md`
- Create: safe screenshots under `docs/validation/evidence/` only when they contain no private content.

1. After the existing VM slot is released, create a fresh lab run and copy the exact source revision into its Tick-only inbox.
2. Build the ARM64 app and NSIS installer, install for the current user, launch without elevation and record versions/hashes.
3. Create a Tick-owned test task using a path with spaces, Chinese and `&`; enable, run, inspect exact stdout/stderr/cwd/argv, then stop a long-running child and prove it is gone.
4. Verify close-to-tray, left-click toggle, right-click menu, menu exit and relaunch.
5. Delete only the acceptance task, prove unrelated sentinel tasks remain, uninstall Tick and save sanitized screenshots.
6. Run the final macOS checks, review the task diff, stage only task paths and commit without publishing a release.
