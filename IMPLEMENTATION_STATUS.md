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

## CURRENT

Milestone 1 is complete and packaged. The approved sequence now continues with Milestone 2: Photo Object.

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
- Existing Ghost state transitions and invalid-setting recovery.

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
- Rust tests: 11 passed.
- Vite production build, rustfmt, and strict Clippy: passed.
- Release executable, MSI, and NSIS installer: produced successfully.

## LIMITATIONS

- NOT PHYSICALLY TESTED: cross-monitor drag/restart, mixed 100%/125%/150% monitor combinations, negative-origin hardware, monitor unplug/reconnect, or sleep/wake display reconfiguration. Only one physical display was available.
- NOT NATIVE TESTED in this pass: tray recovery interaction; the tray implementation was unchanged.
- Automated Windows input could not sustain the native move/resize handle drag gesture, so the existing direct edit-handle gesture was not revalidated; Studio-driven size persistence and restart were verified instead.
- Autostart was not enabled because that would change the user's Windows startup configuration.

## SOURCE CONTROL

Repository: https://github.com/uges24/see-me-
Branch: main
Baseline committed: yes
Remote tracking: origin/main

## NEXT

### Milestone 2 — Photo Object

Add a local PNG/JPEG/WebP as a second borderless native desktop object. Evolve the runtime toward shared `DesktopObject + ObjectType + Behaviour` primitives so Clock and Photo reuse geometry, monitor recovery, persistence, always-on-top, Stay/Ghost/Fade/Click Through, and Edit/Live behavior. Copy imported media into application storage; never depend on a temporary browser blob URL or modify the original file.

## Build artifacts

- `src-tauri/target/release/app.exe`
- `src-tauri/target/release/bundle/msi/Timepiece Studio_0.1.0_x64_en-US.msi`
- `src-tauri/target/release/bundle/nsis/Timepiece Studio_0.1.0_x64-setup.exe`
