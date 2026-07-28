# Tick Open-source Refresh Implementation Plan

> **For Claude:** REQUIRED SUB-SKILL: Use executing-plans to implement this plan task-by-task.

**Goal:** Turn Tick into a polished, public learning project with an original clock mascot, clearer macOS UI, honest LaunchAgent-focused documentation, and product screenshots.

**Architecture:** Keep the existing Tauri, React, TypeScript, and Rust implementation intact. Refresh only presentation assets, app-shell styling, open-source documentation, and repository metadata; preserve all task-management behavior and the user's existing uncommitted UI work.

**Tech Stack:** Tauri 2, React 19, TypeScript, Ant Design, Rust, GitHub Markdown.

---

### Task 1: Establish the visual identity

**Files:**
- Create: `docs/images/tick-mascot-source.png`
- Create: `src/assets/tick-mascot.png`
- Modify: `src-tauri/icons/*`

1. Generate an original teal twin-tail clock mascot on a removable flat background.
2. Remove the background and validate alpha edges.
3. Create the macOS application icon and platform icon sizes from the selected asset.
4. Keep the original generated source in `docs/images` for traceability.

### Task 2: Refresh the application chrome

**Files:**
- Modify: `src/App.tsx`
- Modify: `src/App.css`
- Modify: `src/components/MikuMascot.tsx`

1. Add a compact brand mark and honest LaunchAgent-oriented subtitle.
2. Refine the dark macOS palette, spacing, panels, and button hierarchy.
3. Rework the empty state around learning and creating the first LaunchAgent.
4. Preserve all existing task actions and data flow.
5. Run `npm run build`.

### Task 3: Create product screenshots

**Files:**
- Create: `docs/images/tick-overview.png`
- Create: `docs/images/tick-create-job.png`
- Create: `docs/images/tick-logs.png`

1. Launch the app or preview with representative local data.
2. Capture the overview, task editor, and logs views.
3. Verify screenshots are readable at GitHub README width.

### Task 4: Prepare the public repository

**Files:**
- Modify: `README.md`
- Create: `LICENSE`
- Create: `CONTRIBUTING.md`
- Create: `SECURITY.md`
- Modify: `.gitignore`

1. Follow the concise product-first structure used by `mimi` and `pyfl`.
2. State clearly that Tick began as a product-shaped experiment for learning LaunchAgent.
3. Document supported behavior, system impact, installation, development, and limitations.
4. Add MIT licensing, contribution guidance, and private security reporting guidance.
5. Ensure build caches and local agent worktrees cannot be published.

### Task 5: Verify and publish

1. Run the frontend build.
2. Run Rust formatting checks and tests.
3. Review the complete Git diff and confirm no secrets or build artifacts are staged.
4. Commit the intended files and push the branch.
5. Merge or publish to the default branch as appropriate.
6. Change `yuxino/tick` visibility from private to public.
7. Verify the public repository page and README rendering.
