# Tick Editorial Console Redesign

Date: 2026-08-24

## Intent

Replace the current layered dashboard with a single, coherent macOS scheduling console. The redesign keeps Tick's launchd data model and proven task actions, but discards the existing page structure and visual CSS.

The interface should feel like a precise operating manual: warm paper, dark ink, fine rules, disciplined typography, and one restrained mint accent inherited from the Tick mascot. Time and commands are the visual material; decoration is not.

## Chosen direction

The page becomes a three-part instrument:

1. A fixed left control rail for brand, task/calendar navigation, live task counts, settings, and the primary create action.
2. A task ledger that supports fast scanning and selection without a wall of cards.
3. A large inspector that gives the selected task a clear schedule, command specimen, direct controls, metadata, logs, and plist.

The calendar uses the same paper-and-rule language as the task view. It remains conditionally mounted so the live log poller does not continue in the background.

## Information architecture

### Control rail

- Preserve a safe top-left area for macOS traffic lights.
- Keep a dedicated drag region that does not cover interactive controls.
- Show the Tick mark and wordmark at compact scale.
- Offer two primary destinations: Tasks and Calendar.
- Summarize enabled, paused, and attention-needed counts.
- Keep Settings and New Task consistently available.

### Task ledger

- Search across name, description, label, schedule, and command.
- Present each task as a numbered ledger row, not a floating card.
- Show status, name, label, and schedule in a compact hierarchy.
- Keep enable/disable and Run Now directly available.
- Preserve keyboard selection with Enter and Space.
- Make the selected row unmistakable through contrast and a thin accent mark.

### Task inspector

- Lead with the task identity and a large schedule display.
- Show the execution command as a readable code specimen.
- Group enable, Run Now, Edit, and Delete into one action rail.
- Keep Overview, Live Logs, and plist as tabs.
- Reset to Overview when the selected task changes.
- Use a flat metadata ledger with copyable labels and paths.

### Calendar

- Use a continuous planner grid with fine rules and no card containers.
- Keep previous month, Today, and next month controls.
- Show enabled task count as supporting context; avoid presenting interval jobs as a misleading monthly run total.
- Render task events as concise schedule marks and open their inspector on selection.

### Empty and error states

- Keep error feedback visible near the top of the working surface.
- Use the mascot sparingly in the empty state.
- Offer both conversational creation and manual entry.

## Visual system

- Canvas: warm white and paper neutrals.
- Text: near-black ink with neutral secondary copy.
- Accent: desaturated mint used only for selected, enabled, focus, and primary action states.
- Semantic colors: amber for missing/warning and red for errors/destructive actions.
- Typography: Avenir Next and PingFang SC for interface copy; Iowan Old Style for oversized editorial numerals and headings; SF Mono for labels, commands, paths, and time.
- Geometry: square-to-soft corners, thin 1px rules, no gradient, no glass, no decorative grid, and almost no shadow.
- Motion: 120–220ms opacity/position transitions for selection and view entry; disabled under reduced-motion.

## Responsive and native constraints

- Optimize for the default 1120×760 window and support the 900×640 minimum.
- At narrower widths, compress the rail and reduce inspector typography before changing the information hierarchy.
- Give the ledger and inspector independent vertical scrolling.
- Preserve CodeMirror and job-editor shrink/overflow constraints for long scripts.
- Keep Ant Design overlays globally aligned through ConfigProvider tokens and modal/popover CSS.

## Scope boundaries

- Preserve launchd service calls, data types, state ownership, and task action handlers.
- Preserve JobFormModal, AutomationModal, and SettingsModal behavior.
- Do not refactor the state layer or backend in this visual pass.
- Remove unused JobsTable edit/delete props if the actions remain inspector-only.

## Acceptance checks

- Production frontend build succeeds.
- Rust formatting and tests succeed.
- Native app is visually checked at default and minimum window sizes.
- Tasks, calendar, task selection, enable/disable, Run Now, edit, delete confirmation, logs, plist, AI creation, manual creation, and settings remain reachable.
- Long labels, commands, and script editor content do not overflow their containers.
- Keyboard focus is visible and reduced-motion is honored.
