# Timepiece Studio — Implementation Status

Updated: 2026-08-18

This file is the tactical handoff. Product intent and architecture belong in `NORMAL.md`; operating rules belong in `AGENTS.md`.

## DONE

### Milestone 1 — Multi-monitor and mixed-DPI hardening

- Kept physical virtual-desktop pixels as the canonical native coordinate system; valid negative X/Y coordinates remain valid.
- Added normalized monitor-relative X/Y persistence alongside absolute position, physical size, monitor name, and scale factor.
- Updated monitor identity, scale factor, and relative placement whenever the native window moves or resizes.
- Made native geometry authoritative so full Studio settings snapshots cannot erase current monitor/DPI/relative placement with stale browser state.
- Restored saved objects against monitor work areas rather than full display bounds, avoiding taskbar-covered placement.
- Preserved relative placement when a known monitor changes resolution or scale.
- Recovered fully offscreen objects and objects saved on a removed monitor onto the primary available work area with a safe margin.
- Added a one-second display-topology signature check and offscreen recovery path for resolution changes, disconnect/reconnect, and display refresh after wake.
- Kept the physical clock size stable across DPI changes and made initial logical sizing an explicit physical-to-logical conversion.
- Preserved the existing physical cursor/window Ghost hit test, including negative coordinates.
- Expanded Rust geometry/behavior coverage from 5 to 11 tests.

### Milestone 2 — Photo Object implementation

- Added `DesktopObjectSettings` and `ObjectType` so Clock and Photo share geometry, behavior, monitor recovery, always-on-top, visibility, lock, and Edit/Live state.
- Added a second transparent, borderless, taskbar-free native Photo window.
- Added local PNG/JPEG/WebP import with MIME/signature validation and a 20 MB limit.
- Copies imported media into the app configuration `objects` directory; the original is never modified and no network request is made.
- Preserves natural photo aspect ratio during creation, native resizing, monitor recovery, and image replacement.
- Added independent Photo behavior/visibility/topmost controls in Studio and minimal move/resize/finish/hide controls visible only in Edit Mode.
- Added debounced Photo JSON persistence and restart loading using the shared placement/recovery primitives.
- Fixed generic recovery so non-square desktop objects are no longer forced square.

### Milestone 2 — Product structure and object controls

- Replaced the mixed long settings surface with Home, Objects, Clock Faces, and app-wide Settings.
- Added a prominent `+ Add object` flow with Clock and Photo; Note and Timer remain disabled future placeholders.
- Added a direct per-object inspector for visibility behavior, independent click-through, topmost, lock, visibility, size, Clock hands/movement/face, Photo replacement, and removal.
- Split hover visibility (`Hide`, `Fade`, `Do nothing`) from persistent pointer pass-through in the shared Rust runtime and migrated legacy saved values.
- Added floating-object settings buttons that open the matching inspector in Studio.
- Removed the photo-only Love Frame and checkerboard Daydream Coast from the Clock Faces gallery.
- Replaced Aurora Orrery and Tangerine Tide with true-alpha production assets and confirmed transparent corner pixels on all four shipped faces.
- Fixed the Tangerine Tide `undefined` render path.

## CURRENT

Clock and Photo now have clear object management and shared native controls. The Photo lifecycle is native-owned: Studio only edits its persisted record and never creates, hides, or destroys the Photo during page navigation. The restart and Studio-interaction regression is fixed; Sticky Note and Timer must not start before the remaining direct manipulation acceptance is closed.

## TESTED

### UNIT TESTED

- Primary and secondary work areas.
- Displays left of and above the primary monitor with negative X/Y.
- Largest-overlap monitor selection.
- Physical/logical conversion at 100%, 125%, and 150% without round-trip drift.
- Removed-monitor fallback to primary.
- Reduced resolution with normalized relative restoration.
- Partial and complete offscreen recovery.
- Square/aspect preservation.
- Physical cursor hit-testing under mixed DPI.
- Existing Hide/Fade state transitions and invalid-setting recovery.
- Independent persistent click-through composed with every visibility behavior.

### NATIVE TESTED — one physical display at 125%

- Development app launched as separate Studio and transparent clock windows.
- Clock size changed and restored after native app rebuild/restart.
- Persisted JSON retained physical size, monitor identity, 1.25 scale, and normalized relative placement.
- A later Studio behavior change no longer erased native relative placement.
- `Ctrl+Shift+E` entered and exited Edit Mode.
- Ghost became invisible and click-through, the underlying Studio accepted the click, and the clock returned after pointer exit.
- Fade rendered at configured opacity, remained click-through, and returned after pointer exit.

### AUTOMATED / PACKAGING

- `npm run verify:full`: passed.
- Node tests: 1 passed.
- Rust tests: 15 passed in the current full verification (the original hardening milestone added 11 geometry/behavior cases).
- Vite production build, rustfmt, and strict Clippy: passed.
- Release executable, MSI, and NSIS installer: produced successfully.

### PHOTO — UNIT / BUILD TESTED

- Supported PNG/JPEG/WebP data signatures and forged MIME/signature rejection.
- Natural-aspect creation and bounded monitor recovery.
- Shared behavior, geometry, and photo import regression suite: 15 Rust tests passed.
- `npm run verify:full`: passed after Photo implementation.
- Release executable, MSI, and NSIS installer rebuilt successfully.
- Persisted app-local Photo reload, missing-asset recovery, and Clock-document/Photo-document isolation are covered by Rust tests.

### PHOTO — NATIVE TESTED

- Development application launched successfully with separate Studio, Clock, and Photo windows.
- An existing imported local Photo rendered in its own always-on-top native window and restored after a full process restart.
- Home and Objects exposed both active objects with direct Edit, Show/Hide, and Remove actions.
- Clock and Photo inspectors exposed the required direct-language controls.
- Clock Show/Hide and Second hand changed the native Clock immediately; the baseline was restored after testing.
- Clock click-through toggled ON independently while Hide on hover remained selected, then returned to OFF.
- Photo click-through toggled ON independently; hover visibility changed from Hide to Fade while click-through stayed ON; both settings were then restored to Hide/OFF.
- Photo size changed from 420 px to 541 px through the inspector; the native window grew from 336×189 to 433×243 logical pixels while preserving its 16:9 aspect ratio, then returned to the original range.
- The Windows local file picker opened from both `+ Add object → Photo`/`Change photo` paths and filtered PNG/JPEG/WebP.
- Regression: importing a Photo after a previously removed Photo record reactivates `enabled` and `visible` before the native window is created. The Photo record is written immediately to app-local storage instead of waiting for the debounce worker.
- Regression: fully stopped and relaunched the real debug app with an imported Photo. The same persisted app-local asset, position, size, Hide behavior, click-through preference, and topmost state restored in a native Photo window.
- Regression: navigated Studio Home → Settings → Clock Faces → Objects and changed the Clock second-hand setting. The native Photo window stayed alive throughout and the Studio reloaded the active Photo state correctly.

## LIMITATIONS

- NOT PHYSICALLY TESTED: cross-monitor drag/restart, mixed 100%/125%/150% monitor combinations, negative-origin hardware, monitor unplug/reconnect, or sleep/wake display reconfiguration. Only one physical display was available.
- NOT NATIVE TESTED in this pass: tray recovery interaction; the tray implementation was unchanged.
- Automated Windows input could not sustain the native move/resize handle drag gesture, so the existing direct edit-handle gesture was not revalidated; Studio-driven size persistence and restart were verified instead.
- Autostart was not enabled because that would change the user's Windows startup configuration.
- NOT NATIVE TESTED in this pass: a fresh file-picker confirmation, direct Photo edit-handle movement, and cross-application click delivery underneath Photo. Inspector-driven native resize and aspect preservation passed, but automated drag injection could not sustain the OS move gesture. Windows exposed the owned file dialog but its automation hit-testing misclassified the dialog surface as the Studio WebView.

## SOURCE CONTROL

Repository: https://github.com/uges24/see-me-
Branch: main
Baseline committed: yes
Remote tracking: origin/main

## NEXT

### Finish the final Milestone 2 direct-manipulation acceptance

Directly drag and resize the Photo in Edit mode and click an underlying normal application with `Let clicks pass through` enabled. Confirm aspect preservation and one more restart restore. Do not start Sticky Note or Timer as part of this task.

## Build artifacts

- `src-tauri/target/release/app.exe`
- `src-tauri/target/release/bundle/msi/Timepiece Studio_0.1.0_x64_en-US.msi`
- `src-tauri/target/release/bundle/nsis/Timepiece Studio_0.1.0_x64-setup.exe`
