mod runtime;

use runtime::{
    BehaviourEngine, BehaviourInput, BehaviourMode, Bounds, InteractionState, RuntimeSettings,
    MAX_SIZE, MIN_SIZE,
};
use serde::Serialize;
use std::{
    fs,
    path::{Path, PathBuf},
    sync::{
        atomic::{AtomicBool, Ordering},
        mpsc::{self, Sender},
        Arc, Mutex,
    },
    thread,
    time::{Duration, Instant},
};
use tauri::{
    menu::{Menu, MenuItem, PredefinedMenuItem},
    tray::TrayIconBuilder,
    Emitter, Manager, PhysicalPosition, PhysicalSize, WebviewUrl, WebviewWindow,
    WebviewWindowBuilder, WindowEvent,
};
use tauri_plugin_autostart::{MacosLauncher, ManagerExt as AutostartExt};
use tauri_plugin_global_shortcut::{Code, GlobalShortcutExt, Modifiers, Shortcut, ShortcutState};

const CLOCK_LABEL: &str = "clock";
const SETTINGS_FILE: &str = "runtime.json";

struct RuntimeState {
    settings: Arc<Mutex<RuntimeSettings>>,
    editing: AtomicBool,
    quitting: AtomicBool,
    save_tx: Sender<()>,
}

#[derive(Clone, Serialize, PartialEq)]
#[serde(rename_all = "camelCase")]
struct Appearance {
    state: InteractionState,
    opacity: f64,
    duration_ms: u64,
    click_through: bool,
    cursor_inside_bounds: bool,
}

#[cfg(debug_assertions)]
#[derive(Clone, Serialize)]
#[serde(rename_all = "camelCase")]
struct DebugSnapshot {
    cursor_x: f64,
    cursor_y: f64,
    window_x: i32,
    window_y: i32,
    width: u32,
    height: u32,
    cursor_inside_bounds: bool,
    state: InteractionState,
    ignore_cursor_events: bool,
    monitor: Option<String>,
    scale_factor: f64,
}

fn settings_path(app: &tauri::AppHandle) -> PathBuf {
    app.path()
        .app_config_dir()
        .unwrap_or_else(|_| PathBuf::from("."))
        .join(SETTINGS_FILE)
}

fn load_settings(path: &Path) -> RuntimeSettings {
    fs::read_to_string(path)
        .ok()
        .and_then(|text| serde_json::from_str::<RuntimeSettings>(&text).ok())
        .unwrap_or_default()
        .validated()
}

fn save_settings(path: &Path, settings: &RuntimeSettings) -> Result<(), String> {
    if let Some(parent) = path.parent() {
        fs::create_dir_all(parent).map_err(|error| error.to_string())?;
    }
    let json = serde_json::to_vec_pretty(settings).map_err(|error| error.to_string())?;
    fs::write(path, json).map_err(|error| error.to_string())
}

fn start_persistence_worker(path: PathBuf, settings: Arc<Mutex<RuntimeSettings>>) -> Sender<()> {
    let (tx, rx) = mpsc::channel();
    thread::spawn(move || {
        while rx.recv().is_ok() {
            while rx.recv_timeout(Duration::from_millis(300)).is_ok() {}
            if let Ok(current) = settings.lock() {
                let _ = save_settings(&path, &current);
            }
        }
    });
    tx
}

fn emit_settings(app: &tauri::AppHandle, settings: &RuntimeSettings) {
    let _ = app.emit_to(CLOCK_LABEL, "runtime-settings", settings.clone());
    let _ = app.emit_to("main", "runtime-settings", settings.clone());
}

fn apply_window_settings(window: &WebviewWindow, settings: &RuntimeSettings) {
    let _ = window.set_always_on_top(settings.always_on_top);
    if settings.visible {
        let _ = window.show();
    } else {
        let _ = window.hide();
    }
}

fn recover_window_to_visible_monitor(window: &WebviewWindow, settings: &mut RuntimeSettings) {
    let monitors = window.available_monitors().unwrap_or_default();
    if monitors.is_empty() {
        return;
    }
    let intersects = monitors.iter().any(|monitor| {
        let position = monitor.position();
        let size = monitor.size();
        settings.x < position.x + size.width as i32
            && settings.x + settings.width as i32 > position.x
            && settings.y < position.y + size.height as i32
            && settings.y + settings.height as i32 > position.y
    });
    let target = monitors
        .iter()
        .find(|monitor| monitor.name() == settings.monitor.as_ref())
        .unwrap_or(&monitors[0]);
    if !intersects {
        settings.x = target.position().x + 40;
        settings.y = target.position().y + 40;
    }
    let position = target.position();
    let size = target.size();
    let max_x = position.x + size.width as i32 - settings.width as i32;
    let max_y = position.y + size.height as i32 - settings.height as i32;
    settings.x = settings.x.clamp(position.x, max_x.max(position.x));
    settings.y = settings.y.clamp(position.y, max_y.max(position.y));
    settings.monitor = target.name().cloned();
    settings.scale_factor = target.scale_factor();
}

fn create_clock_window(
    app: &tauri::App,
    settings: Arc<Mutex<RuntimeSettings>>,
    save_tx: Sender<()>,
) -> tauri::Result<WebviewWindow> {
    let current = settings.lock().expect("runtime settings poisoned").clone();
    let window = WebviewWindowBuilder::new(
        app,
        CLOCK_LABEL,
        WebviewUrl::App("index.html?window=clock".into()),
    )
    .title("Timepiece Clock")
    .inner_size(current.width as f64, current.height as f64)
    .min_inner_size(MIN_SIZE as f64, MIN_SIZE as f64)
    .max_inner_size(MAX_SIZE as f64, MAX_SIZE as f64)
    .transparent(true)
    .decorations(false)
    .shadow(false)
    .resizable(false)
    .always_on_top(current.always_on_top)
    .skip_taskbar(true)
    .focused(false)
    .focusable(false)
    .visible(current.visible)
    .build()?;
    {
        let mut current = settings.lock().expect("runtime settings poisoned");
        recover_window_to_visible_monitor(&window, &mut current);
        let _ = window.set_position(PhysicalPosition::new(current.x, current.y));
        let _ = window.set_size(PhysicalSize::new(current.width, current.height));
    }
    let event_settings = settings.clone();
    let event_tx = save_tx.clone();
    let resize_window = window.clone();
    window.on_window_event(move |event| match event {
        WindowEvent::Moved(position) => {
            if let Ok(mut current) = event_settings.lock() {
                current.x = position.x;
                current.y = position.y;
            }
            let _ = event_tx.send(());
        }
        WindowEvent::Resized(size) => {
            if size.width != size.height {
                let side = size.width.max(size.height).clamp(MIN_SIZE, MAX_SIZE);
                let _ = resize_window.set_size(PhysicalSize::new(side, side));
                return;
            }
            if let Ok(mut current) = event_settings.lock() {
                current.width = size.width.clamp(MIN_SIZE, MAX_SIZE);
                current.height = current.width;
            }
            let _ = event_tx.send(());
        }
        WindowEvent::CloseRequested { api, .. } => api.prevent_close(),
        _ => {}
    });
    Ok(window)
}

fn toggle_edit_internal(app: &tauri::AppHandle) {
    let runtime = app.state::<RuntimeState>();
    let editing = !runtime.editing.fetch_xor(true, Ordering::SeqCst);
    if let Some(clock) = app.get_webview_window(CLOCK_LABEL) {
        let _ = clock.set_ignore_cursor_events(false);
        let _ = clock.set_focusable(editing);
        let _ = clock.set_resizable(editing);
        let _ = clock.show();
        if editing {
            let _ = clock.set_focus();
        }
    }
    let _ = app.emit_to(CLOCK_LABEL, "edit-mode", editing);
    let _ = app.emit_to("main", "edit-mode", editing);
}

fn start_behaviour_engine(app: tauri::AppHandle) {
    thread::spawn(move || {
        let mut engine = BehaviourEngine::default();
        let mut previous: Option<Appearance> = None;
        let mut previous_tick = Instant::now();
        let mut last_emit = Instant::now() - Duration::from_secs(2);
        loop {
            let runtime = app.state::<RuntimeState>();
            if runtime.quitting.load(Ordering::Relaxed) {
                break;
            }
            let settings = runtime
                .settings
                .lock()
                .map(|item| item.clone())
                .unwrap_or_default();
            let editing = runtime.editing.load(Ordering::Relaxed);
            let Some(clock) = app.get_webview_window(CLOCK_LABEL) else {
                thread::sleep(Duration::from_millis(250));
                continue;
            };
            if !settings.visible {
                thread::sleep(Duration::from_millis(250));
                previous_tick = Instant::now();
                continue;
            }
            let position = clock.outer_position().unwrap_or_default();
            let size = clock
                .outer_size()
                .unwrap_or(PhysicalSize::new(settings.width, settings.height));
            let cursor = clock.cursor_position().unwrap_or_default();
            let inside = Bounds {
                x: position.x,
                y: position.y,
                width: size.width,
                height: size.height,
            }
            .contains(cursor.x, cursor.y);
            let now = Instant::now();
            let dt = now.saturating_duration_since(previous_tick);
            previous_tick = now;
            let output = engine.step(BehaviourInput {
                mode: settings.behaviour,
                inside,
                editing,
                dt,
                hide_delay: Duration::from_millis(settings.ghost_hide_delay),
                return_delay: Duration::from_millis(settings.ghost_return_delay),
                fade_opacity: settings.fade_opacity,
            });
            let duration_ms = match output.state {
                InteractionState::Hiding => 140,
                InteractionState::Showing => 190,
                _ => 0,
            };
            let appearance = Appearance {
                state: output.state,
                opacity: output.opacity,
                duration_ms,
                click_through: output.click_through,
                cursor_inside_bounds: inside,
            };
            if previous.as_ref() != Some(&appearance)
                || last_emit.elapsed() >= Duration::from_secs(1)
            {
                let _ = clock.set_ignore_cursor_events(output.click_through && !editing);
                let _ = app.emit_to(CLOCK_LABEL, "clock-appearance", appearance.clone());
                last_emit = Instant::now();
                #[cfg(debug_assertions)]
                {
                    let monitor = clock.current_monitor().ok().flatten();
                    let snapshot = DebugSnapshot {
                        cursor_x: cursor.x,
                        cursor_y: cursor.y,
                        window_x: position.x,
                        window_y: position.y,
                        width: size.width,
                        height: size.height,
                        cursor_inside_bounds: inside,
                        state: output.state,
                        ignore_cursor_events: output.click_through && !editing,
                        monitor: monitor.as_ref().and_then(|item| item.name().cloned()),
                        scale_factor: clock.scale_factor().unwrap_or(1.0),
                    };
                    let _ = app.emit_to(CLOCK_LABEL, "debug-snapshot", snapshot);
                }
                previous = Some(appearance);
            }
            let active = editing
                || matches!(
                    settings.behaviour,
                    BehaviourMode::Ghost | BehaviourMode::Fade
                );
            thread::sleep(if active {
                Duration::from_millis(33)
            } else {
                Duration::from_millis(250)
            });
        }
    });
}

#[tauri::command]
fn get_settings(state: tauri::State<RuntimeState>) -> RuntimeSettings {
    state
        .settings
        .lock()
        .map(|item| item.clone())
        .unwrap_or_default()
}

#[tauri::command]
fn update_settings(
    app: tauri::AppHandle,
    state: tauri::State<RuntimeState>,
    settings: RuntimeSettings,
) -> Result<RuntimeSettings, String> {
    let settings = settings.validated();
    if let Some(clock) = app.get_webview_window(CLOCK_LABEL) {
        apply_window_settings(&clock, &settings);
    }
    *state.settings.lock().map_err(|error| error.to_string())? = settings.clone();
    state.save_tx.send(()).map_err(|error| error.to_string())?;
    emit_settings(&app, &settings);
    Ok(settings)
}

#[tauri::command]
fn toggle_edit(app: tauri::AppHandle) {
    toggle_edit_internal(&app);
}

#[tauri::command]
fn start_clock_drag(app: tauri::AppHandle) -> Result<(), String> {
    app.get_webview_window(CLOCK_LABEL)
        .ok_or_else(|| "clock window unavailable".to_string())?
        .start_dragging()
        .map_err(|error| error.to_string())
}

#[tauri::command]
fn resize_clock(app: tauri::AppHandle, size: u32) -> Result<u32, String> {
    let size = size.clamp(MIN_SIZE, MAX_SIZE);
    app.get_webview_window(CLOCK_LABEL)
        .ok_or_else(|| "clock window unavailable".to_string())?
        .set_size(PhysicalSize::new(size, size))
        .map_err(|error| error.to_string())?;
    Ok(size)
}

#[tauri::command]
fn start_clock_resize(app: tauri::AppHandle) -> Result<(), String> {
    app.get_webview_window(CLOCK_LABEL)
        .ok_or_else(|| "clock window unavailable".to_string())?
        .as_ref()
        .window()
        .start_resize_dragging(tauri_runtime::ResizeDirection::SouthEast)
        .map_err(|error| error.to_string())
}

#[tauri::command]
fn set_launch_at_login(
    app: tauri::AppHandle,
    state: tauri::State<RuntimeState>,
    enabled: bool,
) -> Result<bool, String> {
    let manager = app.autolaunch();
    if enabled {
        manager.enable()
    } else {
        manager.disable()
    }
    .map_err(|error| error.to_string())?;
    if let Ok(mut settings) = state.settings.lock() {
        settings.launch_at_login = enabled;
    }
    let _ = state.save_tx.send(());
    Ok(enabled)
}

fn build_tray(app: &tauri::App) -> tauri::Result<()> {
    let open = MenuItem::with_id(app, "open", "Open Studio", true, None::<&str>)?;
    let edit = MenuItem::with_id(app, "edit", "Edit Clock", true, Some("Ctrl+Shift+E"))?;
    let show = MenuItem::with_id(app, "show", "Show / Hide Clock", true, None::<&str>)?;
    let top = MenuItem::with_id(app, "top", "Toggle Always on Top", true, None::<&str>)?;
    let ghost = MenuItem::with_id(app, "ghost", "Ghost on Hover", true, None::<&str>)?;
    let launch = MenuItem::with_id(app, "launch", "Toggle Launch at Login", true, None::<&str>)?;
    let quit = MenuItem::with_id(app, "quit", "Quit Timepiece Studio", true, None::<&str>)?;
    let separator = PredefinedMenuItem::separator(app)?;
    let separator_two = PredefinedMenuItem::separator(app)?;
    let menu = Menu::with_items(
        app,
        &[
            &open,
            &edit,
            &show,
            &separator,
            &top,
            &ghost,
            &launch,
            &separator_two,
            &quit,
        ],
    )?;
    TrayIconBuilder::new()
        .icon(
            app.default_window_icon()
                .expect("application icon missing")
                .clone(),
        )
        .tooltip("Timepiece Studio")
        .menu(&menu)
        .show_menu_on_left_click(false)
        .on_menu_event(|app, event| match event.id.as_ref() {
            "open" => {
                if let Some(main) = app.get_webview_window("main") {
                    let _ = main.show();
                    let _ = main.unminimize();
                    let _ = main.set_focus();
                }
            }
            "edit" => toggle_edit_internal(app),
            "show" => {
                let runtime = app.state::<RuntimeState>();
                if let Ok(mut settings) = runtime.settings.lock() {
                    settings.visible = !settings.visible;
                    if let Some(clock) = app.get_webview_window(CLOCK_LABEL) {
                        apply_window_settings(&clock, &settings);
                    }
                    emit_settings(app, &settings);
                }
                let _ = runtime.save_tx.send(());
            }
            "top" => {
                let runtime = app.state::<RuntimeState>();
                if let Ok(mut settings) = runtime.settings.lock() {
                    settings.always_on_top = !settings.always_on_top;
                    if let Some(clock) = app.get_webview_window(CLOCK_LABEL) {
                        let _ = clock.set_always_on_top(settings.always_on_top);
                    }
                    emit_settings(app, &settings);
                }
                let _ = runtime.save_tx.send(());
            }
            "ghost" => {
                let runtime = app.state::<RuntimeState>();
                if let Ok(mut settings) = runtime.settings.lock() {
                    settings.behaviour = BehaviourMode::Ghost;
                    emit_settings(app, &settings);
                }
                let _ = runtime.save_tx.send(());
            }
            "launch" => {
                let runtime = app.state::<RuntimeState>();
                let enabled = runtime
                    .settings
                    .lock()
                    .map(|item| !item.launch_at_login)
                    .unwrap_or(false);
                let manager = app.autolaunch();
                if (if enabled {
                    manager.enable()
                } else {
                    manager.disable()
                })
                .is_ok()
                {
                    if let Ok(mut settings) = runtime.settings.lock() {
                        settings.launch_at_login = enabled;
                        emit_settings(app, &settings);
                    }
                    let _ = runtime.save_tx.send(());
                }
            }
            "quit" => {
                let runtime = app.state::<RuntimeState>();
                runtime.quitting.store(true, Ordering::SeqCst);
                if let Ok(settings) = runtime.settings.lock() {
                    let _ = save_settings(&settings_path(app), &settings);
                }
                app.exit(0);
            }
            _ => {}
        })
        .build(app)?;
    Ok(())
}

#[cfg_attr(mobile, tauri::mobile_entry_point)]
pub fn run() {
    tauri::Builder::default()
        .plugin(tauri_plugin_autostart::init(
            MacosLauncher::LaunchAgent,
            None,
        ))
        .setup(|app| {
            let path = settings_path(app.handle());
            let settings = Arc::new(Mutex::new(load_settings(&path)));
            let save_tx = start_persistence_worker(path, settings.clone());
            app.manage(RuntimeState {
                settings: settings.clone(),
                editing: AtomicBool::new(false),
                quitting: AtomicBool::new(false),
                save_tx: save_tx.clone(),
            });
            create_clock_window(app, settings, save_tx)?;
            build_tray(app)?;
            let shortcut = Shortcut::new(Some(Modifiers::CONTROL | Modifiers::SHIFT), Code::KeyE);
            let handled_shortcut = shortcut;
            app.handle().plugin(
                tauri_plugin_global_shortcut::Builder::new()
                    .with_handler(move |app, pressed, event| {
                        if pressed == &handled_shortcut && event.state() == ShortcutState::Pressed {
                            toggle_edit_internal(app);
                        }
                    })
                    .build(),
            )?;
            if let Err(error) = app.global_shortcut().register(shortcut) {
                log::warn!("Ctrl+Shift+E unavailable: {error}");
            }
            let main = app.get_webview_window("main").expect("main window missing");
            let main_app = app.handle().clone();
            main.on_window_event(move |event| {
                if let WindowEvent::CloseRequested { api, .. } = event {
                    if !main_app
                        .state::<RuntimeState>()
                        .quitting
                        .load(Ordering::Relaxed)
                    {
                        api.prevent_close();
                        if let Some(window) = main_app.get_webview_window("main") {
                            let _ = window.hide();
                        }
                    }
                }
            });
            start_behaviour_engine(app.handle().clone());
            Ok(())
        })
        .invoke_handler(tauri::generate_handler![
            get_settings,
            update_settings,
            toggle_edit,
            start_clock_drag,
            resize_clock,
            start_clock_resize,
            set_launch_at_login
        ])
        .run(tauri::generate_context!())
        .expect("error while running Timepiece Studio");
}
