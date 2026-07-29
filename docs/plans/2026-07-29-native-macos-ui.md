# Native macOS UI Implementation Plan

> **For Claude:** REQUIRED SUB-SKILL: Use superpowers:executing-plans to implement this plan task-by-task.

**Goal:** Make Tick feel like a quiet macOS utility instead of an AI-generated dashboard.

**Architecture:** Keep the existing React, Ant Design, and Tauri behavior. Simplify the information architecture and add a final, scoped visual layer rather than rewriting working launchd logic.

**Tech Stack:** React 19, TypeScript, Ant Design 6, CSS, Tauri 2.

---

### Task 1: Simplify the main window

**Files:**
- Modify: `src/App.tsx`
- Modify: `src/App.css`

1. Remove the mascot badge, status slogan, subtitle, and separate AI header button.
2. Make “New Task” open the assisted creation flow.
3. Replace the embedded empty-state script editor with a restrained empty state.
4. Verify header balance and empty state at the default window size.

### Task 2: Simplify task creation

**Files:**
- Modify: `src/components/AutomationModal.tsx`
- Modify: `src/components/JobFormModal.tsx`
- Modify: `src/App.css`

1. Present AI as an optional way to create a task, not as the product identity.
2. Add a direct manual-create action.
3. Remove numbered section badges and card-heavy form styling.
4. Show draft risks only when risks exist.
5. Verify manual, generated, and long-code states.

### Task 3: Unify visual language

**Files:**
- Modify: `src/App.css`

1. Remove yellow-green gradients and decorative shadows.
2. Use neutral macOS surfaces, one teal accent, modest radii, and quiet borders.
3. Keep keyboard focus visible and preserve contrast.
4. Run the production build and inspect the packaged app.
