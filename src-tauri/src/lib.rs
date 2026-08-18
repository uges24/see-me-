mod runtime;

use base64::{engine::general_purpose::STANDARD as BASE64, Engine as _};
use runtime::{
    physical_to_logical, BehaviourEngine, BehaviourInput, BehaviourMode, Bounds,
    DesktopObjectSettings, DisplayArea, InteractionState, PhotoSettings, RuntimeSettings, MAX_SIZE,
    MIN_SIZE,
};
use serde::{Deserialize, Serialize};
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
const PHOTO_LABEL: &str = "photo";
const SETTINGS_FILE: &str = "runtime.json";
const PHOTO_SETTINGS_FILE: &str = "photo.json";

struct RuntimeState {
    settings: Arc<Mutex<RuntimeSettings>>,
    photo: Arc<Mutex<Option<PhotoSettings>>>,
    editing: AtomicBool,
    quitting: AtomicBool,
    save_tx: Sender<()>,
    photo_save_tx: Sender<()>,
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

fn settings_path(app: &tauri::AppHandle, filename: &str) -> PathBuf {
    app.path()
        .app_config_dir()
        .unwrap_or_else(|_| PathBuf::from("."))
        .join(filename)
}

fn load_settings(path: &Path) -> RuntimeSettings {
    fs::read_to_string(path)
        .ok()
        .and_then(|text| serde_json::from_str::<RuntimeSettings>(&text).ok())
        .unwrap_or_default()
        .validated()
}

fn load_photo_settings(path: &Path) -> Option<PhotoSettings> {
    fs::read_to_string(path)
        .ok()
        .and_then(|text| serde_json::from_str::<PhotoSettings>(&text).ok())
        .map(PhotoSettings::validated)
        .filter(|settings| Path::new(&settings.asset_path).is_file())
}

fn save_settings<T: serde::Serialize>(path: &Path, settings: &T) -> Result<(), String> {
    if let Some(parent) = path.parent() {
        fs::create_dir_all(parent).map_err(|error| error.to_string())?;
    }
    let json = serde_json::to_vec_pretty(settings).map_err(|error| error.to_string())?;
    fs::write(path, json).map_err(|error| error.to_string())
}

fn start_persistence_worker<T>(path: PathBuf, settings: Arc<Mutex<T>>) -> Sender<()>
where
    T: serde::Serialize + Send + 'static,
{
    let (tx, rx) = mpsc::channel();
    thread::spawn(move || {
        while rx.recv().is_ok() {
            while rx.recv_timeout(Duration::from_millis(300)).is_ok() {}
            if let Ok(current) = settings.lock() {
                let _ = save_settings(&path, &*current);
            }
        }
    });
    tx
}

fn emit_settings(app: &tauri::AppHandle, settings: &RuntimeSettings) {
    let _ = app.emit_to(CLOCK_LABEL, "runtime-settings", settings.clone());
    let _ = app.emit_to("main", "runtime-settings", settings.clone());
}

fn emit_photo_settings(app: &tauri::AppHandle, settings: &PhotoSettings) {
    let _ = app.emit_to(PHOTO_LABEL, "photo-settings", settings.clone());
    let _ = app.emit_to("main", "photo-settings", settings.clone());
}

fn apply_window_settings(window: &WebviewWindow, settings: &DesktopObjectSettings) {
    let _ = window.set_always_on_top(settings.always_on_top);
    let _ = window.set_size(PhysicalSize::new(settings.width, settings.height));
    if settings.visible {
        let _ = window.show();
    } else {
        let _ = window.hide();
    }
}

fn display_areas(window: &WebviewWindow) -> Vec<DisplayArea> {
    window
        .available_monitors()
        .unwrap_or_default()
        .into_iter()
        .map(|monitor| {
            let work_area = monitor.work_area();
            DisplayArea {
                name: monitor.name().cloned(),
                work_area: Bounds {
                    x: work_area.position.x,
                    y: work_area.position.y,
                    width: work_area.size.width,
                    height: work_area.size.height,
                },
                scale_factor: monitor.scale_factor(),
            }
        })
        .collect()
}

fn primary_monitor_name(window: &WebviewWindow) -> Option<String> {
    window
        .primary_monitor()
        .ok()
        .flatten()
        .and_then(|monitor| monitor.name().cloned())
}

fn recover_window_to_visible_monitor(window: &WebviewWindow, settings: &mut DesktopObjectSettings) {
    let displays = display_areas(window);
    let primary = primary_monitor_name(window);
    settings.restore_display_placement(&displays, primary.as_deref());
}

fn display_signature(displays: &[DisplayArea]) -> Vec<(Option<String>, i32, i32, u32, u32, u64)> {
    let mut signature = displays
        .iter()
        .map(|display| {
            (
                display.name.clone(),
                display.work_area.x,
                display.work_area.y,
                display.work_area.width,
                display.work_area.height,
                display.scale_factor.to_bits(),
            )
        })
        .collect::<Vec<_>>();
    signature.sort_by(|a, b| a.0.cmp(&b.0).then(a.1.cmp(&b.1)).then(a.2.cmp(&b.2)));
    signature
}

fn reconcile_object_layout(
    window: &WebviewWindow,
    current: &mut DesktopObjectSettings,
    save_tx: &Sender<()>,
) {
    let displays = display_areas(window);
    let primary = primary_monitor_name(window);
    if current.restore_display_placement(&displays, primary.as_deref()) {
        let _ = window.set_position(PhysicalPosition::new(current.x, current.y));
        let _ = window.set_size(PhysicalSize::new(current.width, current.height));
        let _ = save_tx.send(());
    }
}

fn reconcile_clock_layout(window: &WebviewWindow, runtime: &RuntimeState) {
    if let Ok(mut current) = runtime.settings.lock() {
        reconcile_object_layout(window, &mut current.object, &runtime.save_tx);
    }
}

fn reconcile_photo_layout(window: &WebviewWindow, runtime: &RuntimeState) {
    if let Ok(mut photo) = runtime.photo.lock() {
        if let Some(current) = photo.as_mut() {
            reconcile_object_layout(window, &mut current.object, &runtime.photo_save_tx);
        }
    }
}

fn clock_object_settings(runtime: &RuntimeState) -> Option<DesktopObjectSettings> {
    runtime
        .settings
        .lock()
        .ok()
        .map(|settings| settings.object.clone())
}

fn photo_object_settings(runtime: &RuntimeState) -> Option<DesktopObjectSettings> {
    runtime
        .photo
        .lock()
        .ok()
        .and_then(|photo| photo.as_ref().map(|settings| settings.object.clone()))
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
    .inner_size(
        physical_to_logical(current.width, current.scale_factor),
        physical_to_logical(current.height, current.scale_factor),
    )
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
        let _ = save_tx.send(());
    }
    let event_settings = settings.clone();
    let event_tx = save_tx.clone();
    let resize_window = window.clone();
    window.on_window_event(move |event| match event {
        WindowEvent::Moved(position) => {
            let displays = display_areas(&resize_window);
            if let Ok(mut current) = event_settings.lock() {
                current.x = position.x;
                current.y = position.y;
                current.capture_display_placement(&displays);
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
                current.capture_display_placement(&display_areas(&resize_window));
            }
            let _ = event_tx.send(());
        }
        WindowEvent::ScaleFactorChanged { scale_factor, .. } => {
            if let Ok(mut current) = event_settings.lock() {
                current.scale_factor = *scale_factor;
                current.capture_display_placement(&display_areas(&resize_window));
                let _ = resize_window.set_size(PhysicalSize::new(current.width, current.height));
            }
            let _ = event_tx.send(());
        }
        WindowEvent::CloseRequested { api, .. } => api.prevent_close(),
        _ => {}
    });
    Ok(window)
}

fn create_photo_window(
    app: &tauri::AppHandle,
    photo: Arc<Mutex<Option<PhotoSettings>>>,
    save_tx: Sender<()>,
) -> tauri::Result<WebviewWindow> {
    let current = photo
        .lock()
        .expect("photo settings poisoned")
        .clone()
        .expect("photo window requires settings");
    let window = WebviewWindowBuilder::new(
        app,
        PHOTO_LABEL,
        WebviewUrl::App("index.html?window=photo".into()),
    )
    .title("Timepiece Photo")
    .inner_size(
        physical_to_logical(current.width, current.scale_factor),
        physical_to_logical(current.height, current.scale_factor),
    )
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
        let mut photo = photo.lock().expect("photo settings poisoned");
        if let Some(current) = photo.as_mut() {
            recover_window_to_visible_monitor(&window, &mut current.object);
            let _ = window.set_position(PhysicalPosition::new(current.x, current.y));
            let _ = window.set_size(PhysicalSize::new(current.width, current.height));
            let _ = save_tx.send(());
        }
    }
    let event_photo = photo.clone();
    let event_tx = save_tx.clone();
    let event_window = window.clone();
    window.on_window_event(move |event| match event {
        WindowEvent::Moved(position) => {
            let displays = display_areas(&event_window);
            if let Ok(mut photo) = event_photo.lock() {
                if let Some(current) = photo.as_mut() {
                    current.x = position.x;
                    current.y = position.y;
                    current.capture_display_placement(&displays);
                }
            }
            let _ = event_tx.send(());
        }
        WindowEvent::Resized(size) => {
            if let Ok(mut photo) = event_photo.lock() {
                if let Some(current) = photo.as_mut() {
                    let width = size.width.clamp(MIN_SIZE, MAX_SIZE);
                    let height = (width as f64 / current.aspect_ratio())
                        .round()
                        .clamp(MIN_SIZE as f64, MAX_SIZE as f64)
                        as u32;
                    if size.width != width || size.height != height {
                        let _ = event_window.set_size(PhysicalSize::new(width, height));
                        return;
                    }
                    current.width = width;
                    current.height = height;
                    current.capture_display_placement(&display_areas(&event_window));
                }
            }
            let _ = event_tx.send(());
        }
        WindowEvent::ScaleFactorChanged { scale_factor, .. } => {
            if let Ok(mut photo) = event_photo.lock() {
                if let Some(current) = photo.as_mut() {
                    current.scale_factor = *scale_factor;
                    current.capture_display_placement(&display_areas(&event_window));
                    let _ = event_window.set_size(PhysicalSize::new(current.width, current.height));
                }
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
    for label in [CLOCK_LABEL, PHOTO_LABEL] {
        let Some(window) = app.get_webview_window(label) else {
            continue;
        };
        let _ = window.set_ignore_cursor_events(false);
        let _ = window.set_focusable(editing);
        let _ = window.set_resizable(editing);
        let _ = window.show();
        if editing {
            let _ = window.set_focus();
        }
        let _ = app.emit_to(label, "edit-mode", editing);
    }
    let _ = app.emit_to("main", "edit-mode", editing);
}

fn start_behaviour_engine(
    app: tauri::AppHandle,
    label: &'static str,
    get_settings: fn(&RuntimeState) -> Option<DesktopObjectSettings>,
    reconcile_layout: fn(&WebviewWindow, &RuntimeState),
) {
    thread::spawn(move || {
        let mut engine = BehaviourEngine::default();
        let mut previous: Option<Appearance> = None;
        let mut previous_tick = Instant::now();
        let mut last_emit = Instant::now() - Duration::from_secs(2);
        let mut last_display_check = Instant::now() - Duration::from_secs(2);
        let mut previous_display_signature = None;
        loop {
            let runtime = app.state::<RuntimeState>();
            if runtime.quitting.load(Ordering::Relaxed) {
                break;
            }
            let Some(settings) = get_settings(&runtime) else {
                thread::sleep(Duration::from_millis(250));
                continue;
            };
            let editing = runtime.editing.load(Ordering::Relaxed);
            let Some(window) = app.get_webview_window(label) else {
                thread::sleep(Duration::from_millis(250));
                continue;
            };
            if last_display_check.elapsed() >= Duration::from_secs(1) {
                let displays = display_areas(&window);
                let signature = display_signature(&displays);
                let layout_changed = previous_display_signature
                    .as_ref()
                    .is_some_and(|previous| previous != &signature);
                let offscreen = !settings.intersects_display(&displays);
                if layout_changed || offscreen {
                    reconcile_layout(&window, &runtime);
                }
                previous_display_signature = Some(signature);
                last_display_check = Instant::now();
            }
            if !settings.visible {
                thread::sleep(Duration::from_millis(250));
                previous_tick = Instant::now();
                continue;
            }
            let position = window.outer_position().unwrap_or_default();
            let size = window
                .outer_size()
                .unwrap_or(PhysicalSize::new(settings.width, settings.height));
            let cursor = window.cursor_position().unwrap_or_default();
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
                let _ = window.set_ignore_cursor_events(output.click_through && !editing);
                let _ = app.emit_to(label, "object-appearance", appearance.clone());
                last_emit = Instant::now();
                #[cfg(debug_assertions)]
                {
                    let monitor = window.current_monitor().ok().flatten();
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
                        scale_factor: window.scale_factor().unwrap_or(1.0),
                    };
                    let _ = app.emit_to(label, "debug-snapshot", snapshot);
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
fn get_photo_settings(state: tauri::State<RuntimeState>) -> Option<PhotoSettings> {
    state.photo.lock().ok().and_then(|photo| photo.clone())
}

#[tauri::command]
fn get_photo_data(state: tauri::State<RuntimeState>) -> Result<Option<String>, String> {
    let photo = state.photo.lock().map_err(|error| error.to_string())?;
    let Some(settings) = photo.as_ref() else {
        return Ok(None);
    };
    let bytes = fs::read(&settings.asset_path).map_err(|error| error.to_string())?;
    Ok(Some(format!(
        "data:{};base64,{}",
        settings.mime_type,
        BASE64.encode(bytes)
    )))
}

#[derive(Deserialize)]
#[serde(rename_all = "camelCase")]
struct PhotoImport {
    data_url: String,
    natural_width: u32,
    natural_height: u32,
}

fn decode_photo_data(data_url: &str) -> Result<(&'static str, &'static str, Vec<u8>), String> {
    const MAX_PHOTO_BYTES: usize = 20 * 1024 * 1024;
    let (header, payload) = data_url
        .split_once(',')
        .ok_or_else(|| "invalid photo data".to_string())?;
    let (mime, extension) = match header {
        "data:image/png;base64" => ("image/png", "png"),
        "data:image/jpeg;base64" | "data:image/jpg;base64" => ("image/jpeg", "jpg"),
        "data:image/webp;base64" => ("image/webp", "webp"),
        _ => return Err("choose a PNG, JPEG, or WebP image".into()),
    };
    if payload.len() > MAX_PHOTO_BYTES * 2 {
        return Err("photo is larger than 20 MB".into());
    }
    let bytes = BASE64
        .decode(payload)
        .map_err(|_| "photo data could not be decoded".to_string())?;
    if bytes.len() > MAX_PHOTO_BYTES {
        return Err("photo is larger than 20 MB".into());
    }
    let valid = match mime {
        "image/png" => bytes.starts_with(&[0x89, b'P', b'N', b'G', 0x0D, 0x0A, 0x1A, 0x0A]),
        "image/jpeg" => bytes.starts_with(&[0xFF, 0xD8, 0xFF]),
        "image/webp" => bytes.starts_with(b"RIFF") && bytes.get(8..12) == Some(b"WEBP"),
        _ => false,
    };
    if !valid {
        return Err("file contents do not match the selected image type".into());
    }
    Ok((mime, extension, bytes))
}

#[tauri::command]
fn import_photo(
    app: tauri::AppHandle,
    state: tauri::State<RuntimeState>,
    photo: PhotoImport,
) -> Result<PhotoSettings, String> {
    if photo.natural_width == 0
        || photo.natural_height == 0
        || photo.natural_width > 30_000
        || photo.natural_height > 30_000
    {
        return Err("photo dimensions are invalid".into());
    }
    let (mime, extension, bytes) = decode_photo_data(&photo.data_url)?;
    let directory = app
        .path()
        .app_config_dir()
        .map_err(|error| error.to_string())?
        .join("objects");
    fs::create_dir_all(&directory).map_err(|error| error.to_string())?;
    let path = directory.join(format!("photo.{extension}"));
    fs::write(&path, bytes).map_err(|error| error.to_string())?;
    let mut settings = PhotoSettings::new(
        path.to_string_lossy().into_owned(),
        mime.into(),
        photo.natural_width,
        photo.natural_height,
    );
    if let Ok(current) = state.photo.lock() {
        if let Some(current) = current.as_ref() {
            settings.object = current.object.clone();
            settings.height = (settings.width as f64 / settings.aspect_ratio())
                .round()
                .clamp(MIN_SIZE as f64, MAX_SIZE as f64) as u32;
        }
    }
    *state.photo.lock().map_err(|error| error.to_string())? = Some(settings.clone());
    state
        .photo_save_tx
        .send(())
        .map_err(|error| error.to_string())?;
    if let Some(window) = app.get_webview_window(PHOTO_LABEL) {
        apply_window_settings(&window, &settings.object);
        let _ = window.show();
        let _ = app.emit_to(PHOTO_LABEL, "photo-data", photo.data_url);
    } else {
        create_photo_window(&app, state.photo.clone(), state.photo_save_tx.clone())
            .map_err(|error| error.to_string())?;
    }
    emit_photo_settings(&app, &settings);
    Ok(settings)
}

#[tauri::command]
fn update_photo_settings(
    app: tauri::AppHandle,
    state: tauri::State<RuntimeState>,
    settings: PhotoSettings,
) -> Result<PhotoSettings, String> {
    let mut settings = settings.validated();
    {
        let current = state.photo.lock().map_err(|error| error.to_string())?;
        let current = current
            .as_ref()
            .ok_or_else(|| "photo object unavailable".to_string())?;
        settings.asset_path.clone_from(&current.asset_path);
        settings.mime_type.clone_from(&current.mime_type);
        settings.natural_width = current.natural_width;
        settings.natural_height = current.natural_height;
        settings.x = current.x;
        settings.y = current.y;
        settings.monitor.clone_from(&current.monitor);
        settings.scale_factor = current.scale_factor;
        settings.relative_x = current.relative_x;
        settings.relative_y = current.relative_y;
    }
    settings = settings.validated();
    if let Some(window) = app.get_webview_window(PHOTO_LABEL) {
        apply_window_settings(&window, &settings.object);
    }
    *state.photo.lock().map_err(|error| error.to_string())? = Some(settings.clone());
    state
        .photo_save_tx
        .send(())
        .map_err(|error| error.to_string())?;
    emit_photo_settings(&app, &settings);
    Ok(settings)
}

#[tauri::command]
fn update_settings(
    app: tauri::AppHandle,
    state: tauri::State<RuntimeState>,
    settings: RuntimeSettings,
) -> Result<RuntimeSettings, String> {
    let mut settings = settings.validated();
    {
        let current = state.settings.lock().map_err(|error| error.to_string())?;
        settings.x = current.x;
        settings.y = current.y;
        settings.monitor.clone_from(&current.monitor);
        settings.scale_factor = current.scale_factor;
        settings.relative_x = current.relative_x;
        settings.relative_y = current.relative_y;
    }
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

fn editable_object_window(app: &tauri::AppHandle, label: &str) -> Result<WebviewWindow, String> {
    if !matches!(label, CLOCK_LABEL | PHOTO_LABEL) {
        return Err("unknown desktop object".into());
    }
    app.get_webview_window(label)
        .ok_or_else(|| "desktop object unavailable".to_string())
}

#[tauri::command]
fn start_object_drag(app: tauri::AppHandle, label: String) -> Result<(), String> {
    editable_object_window(&app, &label)?
        .start_dragging()
        .map_err(|error| error.to_string())
}

#[tauri::command]
fn start_object_resize(app: tauri::AppHandle, label: String) -> Result<(), String> {
    editable_object_window(&app, &label)?
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
                    let _ = save_settings(&settings_path(app, SETTINGS_FILE), &*settings);
                }
                if let Ok(photo) = runtime.photo.lock() {
                    let _ = save_settings(&settings_path(app, PHOTO_SETTINGS_FILE), &*photo);
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
            let path = settings_path(app.handle(), SETTINGS_FILE);
            let photo_path = settings_path(app.handle(), PHOTO_SETTINGS_FILE);
            let settings = Arc::new(Mutex::new(load_settings(&path)));
            let photo = Arc::new(Mutex::new(load_photo_settings(&photo_path)));
            let save_tx = start_persistence_worker(path, settings.clone());
            let photo_save_tx = start_persistence_worker(photo_path, photo.clone());
            app.manage(RuntimeState {
                settings: settings.clone(),
                photo: photo.clone(),
                editing: AtomicBool::new(false),
                quitting: AtomicBool::new(false),
                save_tx: save_tx.clone(),
                photo_save_tx: photo_save_tx.clone(),
            });
            create_clock_window(app, settings, save_tx)?;
            if photo.lock().map(|item| item.is_some()).unwrap_or(false) {
                create_photo_window(app.handle(), photo, photo_save_tx)?;
            }
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
            start_behaviour_engine(
                app.handle().clone(),
                CLOCK_LABEL,
                clock_object_settings,
                reconcile_clock_layout,
            );
            start_behaviour_engine(
                app.handle().clone(),
                PHOTO_LABEL,
                photo_object_settings,
                reconcile_photo_layout,
            );
            Ok(())
        })
        .invoke_handler(tauri::generate_handler![
            get_settings,
            update_settings,
            get_photo_settings,
            get_photo_data,
            import_photo,
            update_photo_settings,
            toggle_edit,
            start_clock_drag,
            resize_clock,
            start_clock_resize,
            start_object_drag,
            start_object_resize,
            set_launch_at_login
        ])
        .run(tauri::generate_context!())
        .expect("error while running Timepiece Studio");
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn photo_import_accepts_supported_signatures() {
        let png = format!(
            "data:image/png;base64,{}",
            BASE64.encode([0x89, b'P', b'N', b'G', 0x0d, 0x0a, 0x1a, 0x0a])
        );
        let jpeg = format!(
            "data:image/jpeg;base64,{}",
            BASE64.encode([0xff, 0xd8, 0xff, 0x00])
        );
        let webp = format!("data:image/webp;base64,{}", BASE64.encode(*b"RIFF0000WEBP"));
        assert_eq!(decode_photo_data(&png).unwrap().0, "image/png");
        assert_eq!(decode_photo_data(&jpeg).unwrap().0, "image/jpeg");
        assert_eq!(decode_photo_data(&webp).unwrap().0, "image/webp");
    }

    #[test]
    fn photo_import_rejects_mime_signature_mismatch() {
        let forged = format!(
            "data:image/png;base64,{}",
            BASE64.encode([0xff, 0xd8, 0xff])
        );
        assert!(decode_photo_data(&forged).is_err());
    }
}
