# Timepiece Studio

## One-line idea

> A native layer for things that live on your screen without getting in your way.

## 1. Vision

The clock is the first object, not the limit of the product. The longer-term idea is that your screen has another layer: useful, personal, and ambient objects can live independently above normal applications. They stay present when useful and intelligently yield when the user needs whatever is underneath.

## 2. Core product principle

**Presence without obstruction.**

An object should be available at a glance, then disappear, fade, or pass input through when it becomes an obstacle. Getting this interaction right matters more than shipping a large widget catalog.

## 3. Current product

Timepiece Studio 0.1.0 is a Windows-first Tauri 2 application with:

- The existing dark editorial Studio for viewing the current face, browsing faces, and changing runtime settings.
- Six Studio choices: Koi Nocturne, Aurora Orrery, Verdant Halo, Tangerine Tide, Daydream Coast, and the local-photo Love Frame.
- One real floating native clock window, separate from the Studio, with no title bar, border, taskbar entry, or rectangular background.
- Independent hour, minute, and optional second hands showing local system time.
- Live and Edit modes, native move, bounded square resize, lock, show/hide, and always-on-top.
- Stay, Ghost, Fade, and Click Through behavior modes with hide/return delays and fade opacity.
- A Rust behavior state machine and native desktop cursor geometry that continue working after the window becomes click-through.
- Local JSON persistence for face, physical position/size, monitor/DPI metadata, behavior, movement, visibility, lock, and launch preference.
- Off-screen recovery, system tray controls, and a global `Ctrl+Shift+E` Edit Mode shortcut.
- Optional Windows launch at login.
- A browser preview that preserves the Studio while explaining that native overlays require the desktop app.
- Bundled local assets and no accounts, analytics, telemetry, or cloud storage.
- Release output as a standalone executable, MSI, and NSIS setup executable.

Koi Nocturne has a genuine-alpha production asset. Several secondary generated source images still require the same cleanup before they are suitable as transparent native overlays.

## 4. Core concepts

### Object

Something with its own position, size, appearance, persistence, and lifecycle on the desktop. V0 supports one Clock object.

### Behaviour

How an object responds to the user. Current behaviors are:

- **Stay:** visible and normally interactive.
- **Ghost:** fades completely and becomes click-through while the pointer occupies its desktop bounds.
- **Fade:** fades to a configured opacity and becomes click-through.
- **Click Through:** remains visible while input always passes to the application below.

Potential future behaviors include Dodge, Peek, Edge, and contextual visibility.

### Live Mode

The ambient state: no controls or object chrome; the configured behavior is active.

### Edit Mode

The manipulation state: the object becomes visible and interactive, with compact move, resize, settings/finish, and hide controls. The tray and global shortcut provide recovery.

### Surface / Ink

A future annotation layer above applications. It does not exist in V0.

## 5. Current V0

V0 proves the native clock-object runtime. The acceptance principle is:

> I can keep the clock on my screen throughout normal work and it never becomes annoying.

Native Chrome QA has verified the central story: the Koi clock floated over Chrome, disappeared while the pointer remained inside its original rectangle, allowed Chrome to receive a click underneath, did not require browser hover events, and returned after the pointer left. Transparency, Edit Mode, the global shortcut, 125% DPI geometry, and restart size persistence were also natively observed. See `IMPLEMENTATION_STATUS.md` for exact test scope and limitations.

## 6. Future object types

Roadmap concepts—not current features:

- **Photo:** a personal, family, inspiration, or reference image.
- **Sticky Note:** persistent text anywhere on screen.
- **Timer:** timer, stopwatch, Pomodoro, or interval display.
- **Countdown:** birthday, deadline, launch, event, or dramatic long-horizon visualization.
- **Reference:** a pinned screenshot or image.
- **Web / Live Region:** a carefully sandboxed live-information object if it can remain lightweight and local-first where practical.

## 7. Future ink system

Potential tools: Pen, Highlighter, Arrow, Text, Eraser, Laser/Fading Ink, Blur, Pixelate, Measure, and Spotlight. Drawing should eventually work above any application and become click-through when Draw Mode ends. Possible persistence levels are Momentary, Session, and Pinned.

## 8. Future capture / lasso

A user may eventually draw or lasso part of the screen and convert it into a screenshot, pinned reference, note, blur region, copied image, or potentially watched region. This is roadmap only.

## 9. Contextual objects

Objects may later be associated with a screen, monitor, application, window, website, or workspace. Example:

```text
open Figma → design notes appear
close Figma → those notes disappear
```

This requires a future context engine; it is not implemented.

## 10. Scenes

Future scenes could include Work, Design, Gaming, Personal, Presentation, and Focus. A scene would control which objects are present and might react to application launch, fullscreen mode, monitor connection, or time of day.

## 11. Product architecture

Current implementation:

```text
Studio UI
    ↓ commands/events
Tauri Runtime
    ├── Floating Clock Window
    ├── Behaviour Engine
    ├── Cursor/Window Geometry
    ├── Persistence
    ├── Tray
    ├── Global Shortcut
    └── Autostart
```

Desired extensible model:

```text
Desktop Runtime
│
├── Objects
├── Behaviour Engine
├── Interaction / Ink
├── Persistence
├── Context Engine
└── Studio
```

Only the Clock object and its general behavior foundation exist today. Future work should extend `DesktopObject + Behaviour`, not duplicate behavior logic for each object type.

## 12. Design principles

- **Invisible until needed:** permanent UI chrome is a cost.
- **Object, not widget:** objects should feel like things sitting on the desktop, not mini web panels.
- **Local-first:** keep data and processing on the device by default.
- **Native behavior:** OS-level interactions must actually work; visual simulations are not substitutes.
- **Interaction before quantity:** five excellent objects are better than fifty weak widgets.
- **Recoverable:** tray and shortcuts must prevent invisible/click-through objects from trapping the user.
- **Fast:** idle CPU, memory, polling, rendering, and disk writes matter.

## 13. Roadmap

### NOW

Phase 1 runtime hardening: multi-monitor/DPI persistence, movement/resize feel, asset transparency, recovery, and regression coverage.

### NEXT

After runtime hardening: Photo, Sticky Note, and Timer/Countdown objects, in that order unless evidence changes the priority.

### LATER

Ink, Capture/Lasso, contextual/app-anchored objects, and Scenes.

### MUCH LATER

An object SDK, ecosystem, or marketplace only if the core interaction quality and real user demand justify it.

## 14. Known limitations

- Windows-first; other desktop platforms are not currently packaged or natively QA tested.
- V0 supports one active clock, not multiple objects.
- A second physical monitor and monitor unplug/reconnect have not been hardware tested.
- The saved monitor/scale factor is restored, but the current native move handler does not update monitor identity after crossing displays; this is the strongest next hardening task.
- Koi has genuine alpha; several secondary generated face files contain baked backgrounds.
- Chrome passed native Ghost click-through QA; File Explorer and VS Code were not separately exercised.
- Autostart is implemented but was not enabled during QA to avoid changing the user's startup configuration.
- The browser preview cannot reproduce native overlay behavior.
- Source control is established on the `main` branch at `https://github.com/uges24/see-me-`.

## 15. Definition of product success

The goal is not “lots of widgets.”

> Useful things can remain part of my desktop environment without competing with the work underneath them.

## Development commands

Fast verification:

```powershell
npm run verify
```

Full release verification:

```powershell
npm run verify:full
```

Native development:

```powershell
npm run tauri -- dev
```
