# Timepiece Studio Agent Manual

## SESSION START

1. Read `AGENTS.md` completely.
2. Read `NORMAL.md` completely.
3. Read `IMPLEMENTATION_STATUS.md` completely.
4. Run `git status` before editing. Preserve unrelated changes and never assume untracked files are disposable.
5. Inspect every source file relevant to the task. Do not rely only on documentation.
6. Run `npm run verify` when a baseline is appropriate.
7. Continue from `CURRENT` / `NEXT` in `IMPLEMENTATION_STATUS.md`.

## SESSION END

1. Test what changed and fix failures.
2. Run `npm run verify`; use `npm run verify:full` at a release/milestone boundary.
3. Launch the real Tauri application when native behavior changed.
4. Review `git diff` when Git exists; otherwise inspect the changed files explicitly.
5. Update `IMPLEMENTATION_STATUS.md`.
6. Update `NORMAL.md` when product or architectural truth changed.
7. Record exactly what was unit, visual, native, or hardware tested.
8. Leave one explicit `NEXT` task.

## Product principle

Timepiece Studio is evolving into a system of native desktop objects. The first object is a floating clock.

> The object remains available at a glance but automatically gets out of the user's way when they need whatever is underneath it.

Interaction quality matters more than feature count. Never trade away Ghost behavior, true transparency, native movement, persistence, recovery, or idle performance merely to add features.

## Current architecture

```text
Studio UI (`src/main.js`, `src/style.css`)
    ↓ Tauri commands/events
Native runtime (`src-tauri/src/lib.rs`)
    ↓ creates and controls
Floating Clock Window (`index.html?window=clock`)
    ↓ appearance events + native input policy
Behaviour Engine (`src-tauri/src/runtime.rs`)
    ↓
Tauri/Windows APIs (cursor geometry, native windows, tray, shortcut, autostart)
```

Important files:

- `src/main.js`: Studio screens, settings bridge, clock-window markup, native event listeners, edit controls, and face selection.
- `src/style.css`: existing Studio visual system plus transparent clock-window, hands, and Edit Mode styling.
- `src/clock.js`: pure analogue-clock angle calculations.
- `test/clock.test.js`: frontend clock calculation regression test.
- `src-tauri/src/lib.rs`: Tauri startup, windows, persistence worker, native behavior loop, commands, tray, global shortcut, autostart, and window recovery.
- `src-tauri/src/runtime.rs`: `RuntimeSettings`, behavior/state types, geometry, validation, state machine, and Rust unit tests.
- `src-tauri/tauri.conf.json`: Studio window, bundle, CSP, and build configuration.
- `src-tauri/capabilities/default.json`: Tauri permissions for `main` and `clock`.
- `public/assets/`: bundled clock faces. `koi-nocturne-alpha.png` is the production transparent Koi face.
- `IMPLEMENTATION_STATUS.md`: tactical execution truth, tests, blockers, and next task.
- `NORMAL.md`: product vision, strategic architecture, and roadmap.

The current runtime owns one clock. Keep behavior conceptually generic:

```text
DesktopObject + Behaviour
```

Future examples include `Photo + Ghost`, `Note + Stay`, `Timer + Fade`, and `Reference + ClickThrough`. Do not create clock-specific behavior forks when the generic engine can express the rule.

## Non-negotiable existing behavior

Preserve all of the following unless a task explicitly changes them:

- A separate, transparent, borderless, shadowless, taskbar-hidden floating native window.
- Correct local hour/minute calculations, interpolated hour position, optional second hand, and smooth/tick movement.
- Always-on-top, native window movement, bounded square resizing, Lock, Live Mode, and Edit Mode.
- Stay, Ghost, Fade, and Click Through behavior modes.
- Native cursor tracking after the window ignores cursor input.
- Local debounced persistence, invalid-state validation, and off-screen recovery.
- Tray recovery/actions, `Ctrl+Shift+E`, show/hide, launch at login, and one active clock.
- Selected-face updates from the Studio to the native object.
- Physical virtual-desktop coordinates, monitor-relative restore metadata, work-area recovery, and explicit DPI conversion at UI boundaries.
- Browser preview without unconditional Tauri calls.

## The core Ghost rule

Never implement Ghost Mode using only `mouseenter`, `mouseleave`, `pointerenter`, `pointerleave`, or CSS `:hover`. Once the clock becomes click-through, browser pointer events can stop. `WebviewWindow::cursor_position()` plus native window bounds and the explicit Rust state machine are part of the product architecture.

Ghost is not complete because a toggle exists. It is complete only when the native window fades out, becomes click-through, stays out while the cursor remains in its original bounds, lets the underlying application receive input, and returns after exit without flicker.

## Geometry authority

The native runtime owns object geometry. Persist `x`, `y`, width, and height as physical virtual-desktop pixels; negative coordinates are valid. Also persist normalized placement relative to the containing monitor work area so resolution and scaling changes can restore intelligently. Studio settings must not overwrite native position, monitor, DPI, or relative-placement metadata with stale browser state.

## Autonomous work loop

```text
READ → UNDERSTAND → INSPECT CURRENT STATE → CHOOSE NEXT UNFINISHED TASK
→ IMPLEMENT → RUN TESTS → RUN REAL APP WHEN NATIVE BEHAVIOR CHANGED
→ VISUALLY/FUNCTIONALLY VERIFY → FIX → RETEST → UPDATE DOCS → CONTINUE
```

Do not ask whether to continue, whether to run tests, or whether to take the next already-approved step. Continue until the requested milestone is complete. Stop only for a genuine user preference, unavailable credential/hardware/external access, destructive action outside existing authority, or investigated technical blocker.

## Verification

Fast development loop:

```powershell
npm run verify
```

This runs Node tests, the Vite production build, rustfmt check, Rust tests, and strict Clippy. There is no TypeScript or configured JavaScript linter in this plain-JavaScript repository.

Full milestone/release loop:

```powershell
npm run verify:full
```

This runs the fast loop and builds the release binary plus MSI/NSIS installers. It is intentionally not required after every small edit because a clean Rust release build is slow.

Native development:

```powershell
npm run tauri -- dev
```

## Native feature QA

Browser testing is insufficient when transparency, positioning, resize, always-on-top, behavior, cursor input, tray, shortcut, DPI, monitor recovery, startup, or native windows change. Launch the actual Tauri app.

Permanent Ghost regression:

1. Place the clock over another normal application.
2. Enter Live Mode.
3. Move the pointer inside the clock.
4. Verify the clock disappears and becomes click-through.
5. Click the underlying application and verify it receives the click.
6. Keep the pointer inside and verify the clock does not flicker back.
7. Move outside and verify the clock returns.

Use precise labels in reports: `UNIT TESTED`, `SIMULATED`, `VISUALLY TESTED`, `NATIVE TESTED`, `HARDWARE TESTED`, or `NOT TESTED`.

## Performance, privacy, and visual rules

- This is a background utility. Avoid unnecessary polling, rerenders, WebViews, timers, dependencies, network calls, and per-frame disk writes.
- The native behavior loop currently polls around 30 Hz only when Ghost/Fade/Edit needs it and slows for idle modes. Measure before changing it.
- Keep the product local-first. Do not add analytics, telemetry, authentication, accounts, tracking, cloud storage, or remote processing without explicit approval.
- Do not redesign the Studio unless asked. Avoid generic dashboard styling.
- Live objects have essentially zero chrome; manipulation controls belong in Edit Mode.
- Recovery paths are mandatory because click-through or invisible objects cannot be edited directly.

## Source-control safety

- Run `git status` before editing and preserve unrelated user changes.
- Never discard or reset user work, mass-format unrelated files, or assume untracked files are disposable.
- Keep changes scoped. Inspect diffs before committing and leave the worktree in an understandable state.

## Documentation checkpoint

Every substantial session ends with:

```text
CODE → VERIFY → UPDATE IMPLEMENTATION_STATUS.md
→ CHECK WHETHER NORMAL.md CHANGED → FINAL DIFF/FILE REVIEW
```

Do not create automatic prose rewriting. The agent is responsible for factual documentation.

## Approved progression

```text
Phase 0  Clock Runtime
Phase 1  Runtime Hardening
Phase 2  Photo Object
Phase 3  Sticky Note Object
Phase 4  Timer / Countdown Object
Phase 5  Universal Behaviour Engine
Phase 6  Screen Ink / Drawing
Phase 7  Capture / Lasso
Phase 8  Contextual / App-Anchored Objects
Phase 9  Scenes / Automation
Phase 10 Extensible Object Platform
```

Do not begin a later phase while an earlier critical runtime issue remains. Roadmap changes belong in `NORMAL.md`.
