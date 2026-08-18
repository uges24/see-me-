use serde::{Deserialize, Serialize};
use std::{
    ops::{Deref, DerefMut},
    time::Duration,
};

pub const MIN_SIZE: u32 = 180;
pub const MAX_SIZE: u32 = 720;

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
#[serde(rename_all = "camelCase")]
pub struct DesktopObjectSettings {
    #[serde(default = "default_clock_id")]
    pub id: String,
    #[serde(default)]
    pub object_type: ObjectType,
    pub x: i32,
    pub y: i32,
    pub width: u32,
    pub height: u32,
    pub monitor: Option<String>,
    pub scale_factor: f64,
    #[serde(default)]
    pub relative_x: Option<f64>,
    #[serde(default)]
    pub relative_y: Option<f64>,
    pub always_on_top: bool,
    pub locked: bool,
    pub behaviour: BehaviourMode,
    pub ghost_hide_delay: u64,
    pub ghost_return_delay: u64,
    pub fade_opacity: f64,
    pub visible: bool,
}

fn default_clock_id() -> String {
    "clock".into()
}

impl DesktopObjectSettings {
    pub fn clock() -> Self {
        Self {
            id: default_clock_id(),
            object_type: ObjectType::Clock,
            x: 80,
            y: 80,
            width: 360,
            height: 360,
            monitor: None,
            scale_factor: 1.0,
            relative_x: None,
            relative_y: None,
            always_on_top: true,
            locked: true,
            behaviour: BehaviourMode::Ghost,
            ghost_hide_delay: 0,
            ghost_return_delay: 150,
            fade_opacity: 0.15,
            visible: true,
        }
    }

    pub fn photo() -> Self {
        Self {
            id: "photo".into(),
            object_type: ObjectType::Photo,
            x: 480,
            y: 80,
            width: 420,
            height: 315,
            locked: false,
            ..Self::clock()
        }
    }

    pub fn validated(mut self) -> Self {
        let defaults = match self.object_type {
            ObjectType::Clock => Self::clock(),
            ObjectType::Photo => Self::photo(),
        };
        self.width = self.width.clamp(MIN_SIZE, MAX_SIZE);
        self.height = self.height.clamp(MIN_SIZE, MAX_SIZE);
        self.fade_opacity = if self.fade_opacity.is_finite() {
            self.fade_opacity.clamp(0.05, 0.5)
        } else {
            defaults.fade_opacity
        };
        self.scale_factor = valid_scale(self.scale_factor);
        self.relative_x = self
            .relative_x
            .filter(|value| value.is_finite())
            .map(|value| value.clamp(0.0, 1.0));
        self.relative_y = self
            .relative_y
            .filter(|value| value.is_finite())
            .map(|value| value.clamp(0.0, 1.0));
        self.ghost_hide_delay = self.ghost_hide_delay.min(2_000);
        self.ghost_return_delay = self.ghost_return_delay.min(2_000);
        self
    }
}

#[derive(Debug, Clone, Copy, Default, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "kebab-case")]
pub enum ObjectType {
    #[default]
    Clock,
    Photo,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
#[serde(rename_all = "camelCase")]
pub struct RuntimeSettings {
    pub selected_face: String,
    #[serde(flatten)]
    pub object: DesktopObjectSettings,
    pub show_second_hand: bool,
    pub smooth_movement: bool,
    pub launch_at_login: bool,
}

impl Default for RuntimeSettings {
    fn default() -> Self {
        Self {
            selected_face: "koi".into(),
            object: DesktopObjectSettings::clock(),
            show_second_hand: true,
            smooth_movement: true,
            launch_at_login: false,
        }
    }
}

impl RuntimeSettings {
    pub fn validated(mut self) -> Self {
        let defaults = Self::default();
        if !matches!(
            self.selected_face.as_str(),
            "koi" | "orbit" | "flower" | "amber" | "asap" | "love"
        ) {
            self.selected_face = defaults.selected_face;
        }
        self.object = self.object.validated();
        self.object.object_type = ObjectType::Clock;
        self.object.id = default_clock_id();
        self.object.height = self.object.width;
        self
    }
}

impl Deref for RuntimeSettings {
    type Target = DesktopObjectSettings;
    fn deref(&self) -> &Self::Target {
        &self.object
    }
}

impl DerefMut for RuntimeSettings {
    fn deref_mut(&mut self) -> &mut Self::Target {
        &mut self.object
    }
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
#[serde(rename_all = "camelCase")]
pub struct PhotoSettings {
    #[serde(flatten)]
    pub object: DesktopObjectSettings,
    pub asset_path: String,
    pub mime_type: String,
    pub natural_width: u32,
    pub natural_height: u32,
}

impl PhotoSettings {
    pub fn new(
        asset_path: String,
        mime_type: String,
        natural_width: u32,
        natural_height: u32,
    ) -> Self {
        let aspect = natural_width.max(1) as f64 / natural_height.max(1) as f64;
        let mut object = DesktopObjectSettings::photo();
        object.height = (object.width as f64 / aspect).round() as u32;
        Self {
            object,
            asset_path,
            mime_type,
            natural_width: natural_width.max(1),
            natural_height: natural_height.max(1),
        }
        .validated()
    }

    pub fn validated(mut self) -> Self {
        self.object = self.object.validated();
        self.object.object_type = ObjectType::Photo;
        self.object.id = "photo".into();
        let aspect = self.aspect_ratio();
        self.object.height = (self.object.width as f64 / aspect)
            .round()
            .clamp(MIN_SIZE as f64, MAX_SIZE as f64) as u32;
        self
    }

    pub fn aspect_ratio(&self) -> f64 {
        self.natural_width.max(1) as f64 / self.natural_height.max(1) as f64
    }
}

impl Deref for PhotoSettings {
    type Target = DesktopObjectSettings;
    fn deref(&self) -> &Self::Target {
        &self.object
    }
}

impl DerefMut for PhotoSettings {
    fn deref_mut(&mut self) -> &mut Self::Target {
        &mut self.object
    }
}

#[derive(Debug, Clone, Copy, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "kebab-case")]
pub enum BehaviourMode {
    Stay,
    Ghost,
    Fade,
    ClickThrough,
}

#[derive(Debug, Clone, Copy, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "kebab-case")]
pub enum InteractionState {
    Visible,
    HideDelay,
    Hiding,
    Ghosted,
    ReturnDelay,
    Showing,
    Editing,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct Bounds {
    pub x: i32,
    pub y: i32,
    pub width: u32,
    pub height: u32,
}

impl Bounds {
    pub fn contains(self, x: f64, y: f64) -> bool {
        x >= self.x as f64
            && y >= self.y as f64
            && x < self.x as f64 + self.width as f64
            && y < self.y as f64 + self.height as f64
    }

    fn intersection_area(self, other: Self) -> u64 {
        let width = (self
            .x
            .saturating_add_unsigned(self.width)
            .min(other.x.saturating_add_unsigned(other.width))
            - self.x.max(other.x))
        .max(0) as u64;
        let height = (self
            .y
            .saturating_add_unsigned(self.height)
            .min(other.y.saturating_add_unsigned(other.height))
            - self.y.max(other.y))
        .max(0) as u64;
        width * height
    }
}

#[derive(Debug, Clone, PartialEq)]
pub struct DisplayArea {
    pub name: Option<String>,
    pub work_area: Bounds,
    pub scale_factor: f64,
}

pub fn physical_to_logical(value: u32, scale_factor: f64) -> f64 {
    value as f64 / valid_scale(scale_factor)
}

#[cfg(test)]
pub fn logical_to_physical(value: f64, scale_factor: f64) -> u32 {
    (value * valid_scale(scale_factor))
        .round()
        .clamp(0.0, u32::MAX as f64) as u32
}

fn valid_scale(scale_factor: f64) -> f64 {
    if scale_factor.is_finite() && scale_factor > 0.0 {
        scale_factor
    } else {
        1.0
    }
}

fn relative_axis(position: i32, object_size: u32, area_position: i32, area_size: u32) -> f64 {
    let travel = area_size.saturating_sub(object_size);
    if travel == 0 {
        0.0
    } else {
        ((position - area_position) as f64 / travel as f64).clamp(0.0, 1.0)
    }
}

fn restored_axis(relative: f64, object_size: u32, area_position: i32, area_size: u32) -> i32 {
    area_position
        + (relative.clamp(0.0, 1.0) * area_size.saturating_sub(object_size) as f64).round() as i32
}

fn best_display(bounds: Bounds, displays: &[DisplayArea]) -> Option<usize> {
    displays
        .iter()
        .enumerate()
        .map(|(index, display)| (index, bounds.intersection_area(display.work_area)))
        .filter(|(_, area)| *area > 0)
        .max_by_key(|(_, area)| *area)
        .map(|(index, _)| index)
}

impl DesktopObjectSettings {
    pub fn bounds(&self) -> Bounds {
        Bounds {
            x: self.x,
            y: self.y,
            width: self.width,
            height: self.height,
        }
    }

    pub fn capture_display_placement(&mut self, displays: &[DisplayArea]) -> bool {
        let Some(index) = best_display(self.bounds(), displays) else {
            return false;
        };
        self.capture_on(&displays[index]);
        true
    }

    pub fn intersects_display(&self, displays: &[DisplayArea]) -> bool {
        best_display(self.bounds(), displays).is_some()
    }

    pub fn restore_display_placement(
        &mut self,
        displays: &[DisplayArea],
        primary_name: Option<&str>,
    ) -> bool {
        if displays.is_empty() {
            return false;
        }

        let saved_index = self.monitor.as_deref().and_then(|name| {
            displays
                .iter()
                .position(|display| display.name.as_deref() == Some(name))
        });
        let current_index = best_display(self.bounds(), displays);
        let primary_index = primary_name.and_then(|name| {
            displays
                .iter()
                .position(|display| display.name.as_deref() == Some(name))
        });
        let saved_relative_index = if self.relative_x.is_some() && self.relative_y.is_some() {
            saved_index
        } else {
            None
        };
        let target_index = saved_relative_index
            .or(current_index)
            .or(saved_index)
            .or(primary_index)
            .unwrap_or(0);
        let target = &displays[target_index];
        let max_width = target.work_area.width.clamp(1, MAX_SIZE);
        let max_height = target.work_area.height.clamp(1, MAX_SIZE);
        let fit = (max_width as f64 / self.width.max(1) as f64)
            .min(max_height as f64 / self.height.max(1) as f64)
            .min(1.0);
        self.width = (self.width.max(1) as f64 * fit).round().max(1.0) as u32;
        self.height = (self.height.max(1) as f64 * fit).round().max(1.0) as u32;

        if saved_index == Some(target_index) {
            if let Some(relative) = self.relative_x {
                self.x = restored_axis(
                    relative,
                    self.width,
                    target.work_area.x,
                    target.work_area.width,
                );
            }
            if let Some(relative) = self.relative_y {
                self.y = restored_axis(
                    relative,
                    self.height,
                    target.work_area.y,
                    target.work_area.height,
                );
            }
        } else if current_index.is_none() {
            self.x = target.work_area.x
                + 40.min(target.work_area.width.saturating_sub(self.width) as i32);
            self.y = target.work_area.y
                + 40.min(target.work_area.height.saturating_sub(self.height) as i32);
        }

        let max_x = target
            .work_area
            .x
            .saturating_add_unsigned(target.work_area.width.saturating_sub(self.width));
        let max_y = target
            .work_area
            .y
            .saturating_add_unsigned(target.work_area.height.saturating_sub(self.height));
        self.x = self.x.clamp(target.work_area.x, max_x);
        self.y = self.y.clamp(target.work_area.y, max_y);
        self.capture_on(target);
        true
    }

    fn capture_on(&mut self, display: &DisplayArea) {
        self.monitor = display.name.clone();
        self.scale_factor = valid_scale(display.scale_factor);
        self.relative_x = Some(relative_axis(
            self.x,
            self.width,
            display.work_area.x,
            display.work_area.width,
        ));
        self.relative_y = Some(relative_axis(
            self.y,
            self.height,
            display.work_area.y,
            display.work_area.height,
        ));
    }
}

#[derive(Debug, Clone, Copy)]
pub struct BehaviourEngine {
    pub state: InteractionState,
    elapsed: Duration,
}

#[derive(Debug, Clone, Copy, PartialEq)]
pub struct BehaviourOutput {
    pub state: InteractionState,
    pub opacity: f64,
    pub click_through: bool,
}

#[derive(Debug, Clone, Copy)]
pub struct BehaviourInput {
    pub mode: BehaviourMode,
    pub inside: bool,
    pub editing: bool,
    pub dt: Duration,
    pub hide_delay: Duration,
    pub return_delay: Duration,
    pub fade_opacity: f64,
}

impl Default for BehaviourEngine {
    fn default() -> Self {
        Self {
            state: InteractionState::Visible,
            elapsed: Duration::ZERO,
        }
    }
}

impl BehaviourEngine {
    pub fn step(&mut self, input: BehaviourInput) -> BehaviourOutput {
        if input.editing {
            self.state = InteractionState::Editing;
            self.elapsed = Duration::ZERO;
            return self.output(input.mode, input.fade_opacity);
        }

        if input.mode == BehaviourMode::Stay {
            self.state = InteractionState::Visible;
            self.elapsed = Duration::ZERO;
            return self.output(input.mode, input.fade_opacity);
        }
        if input.mode == BehaviourMode::ClickThrough {
            self.state = InteractionState::Ghosted;
            return self.output(input.mode, input.fade_opacity);
        }

        self.elapsed += input.dt;
        match self.state {
            InteractionState::Editing => {
                self.state = InteractionState::Visible;
                self.elapsed = Duration::ZERO;
            }
            InteractionState::Visible if input.inside => {
                self.state = if input.hide_delay.is_zero() {
                    InteractionState::Hiding
                } else {
                    InteractionState::HideDelay
                };
                self.elapsed = Duration::ZERO;
            }
            InteractionState::HideDelay if !input.inside => {
                self.state = InteractionState::Visible;
                self.elapsed = Duration::ZERO;
            }
            InteractionState::HideDelay if self.elapsed >= input.hide_delay => {
                self.state = InteractionState::Hiding;
                self.elapsed = Duration::ZERO;
            }
            InteractionState::Hiding if !input.inside => {
                self.state = InteractionState::Showing;
                self.elapsed = Duration::ZERO;
            }
            InteractionState::Hiding if self.elapsed >= Duration::from_millis(140) => {
                self.state = InteractionState::Ghosted;
                self.elapsed = Duration::ZERO;
            }
            InteractionState::Ghosted if !input.inside => {
                self.state = if input.return_delay.is_zero() {
                    InteractionState::Showing
                } else {
                    InteractionState::ReturnDelay
                };
                self.elapsed = Duration::ZERO;
            }
            InteractionState::ReturnDelay if input.inside => {
                self.state = InteractionState::Ghosted;
                self.elapsed = Duration::ZERO;
            }
            InteractionState::ReturnDelay if self.elapsed >= input.return_delay => {
                self.state = InteractionState::Showing;
                self.elapsed = Duration::ZERO;
            }
            InteractionState::Showing if input.inside => {
                self.state = InteractionState::Hiding;
                self.elapsed = Duration::ZERO;
            }
            InteractionState::Showing if self.elapsed >= Duration::from_millis(190) => {
                self.state = InteractionState::Visible;
                self.elapsed = Duration::ZERO;
            }
            _ => {}
        }
        self.output(input.mode, input.fade_opacity)
    }

    fn output(self, mode: BehaviourMode, fade_opacity: f64) -> BehaviourOutput {
        let hidden_opacity = if mode == BehaviourMode::Fade {
            fade_opacity
        } else if mode == BehaviourMode::ClickThrough {
            1.0
        } else {
            0.0
        };
        let opacity = match self.state {
            InteractionState::Ghosted => hidden_opacity,
            InteractionState::Hiding => hidden_opacity,
            _ => 1.0,
        };
        BehaviourOutput {
            state: self.state,
            opacity,
            click_through: self.state == InteractionState::Ghosted,
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn display(name: &str, x: i32, y: i32, width: u32, height: u32, scale: f64) -> DisplayArea {
        DisplayArea {
            name: Some(name.into()),
            work_area: Bounds {
                x,
                y,
                width,
                height,
            },
            scale_factor: scale,
        }
    }

    fn tick(engine: &mut BehaviourEngine, inside: bool, ms: u64) -> BehaviourOutput {
        engine.step(BehaviourInput {
            mode: BehaviourMode::Ghost,
            inside,
            editing: false,
            dt: Duration::from_millis(ms),
            hide_delay: Duration::ZERO,
            return_delay: Duration::from_millis(150),
            fade_opacity: 0.15,
        })
    }

    #[test]
    fn geometry_handles_edges_and_negative_monitors() {
        let bounds = Bounds {
            x: -500,
            y: 40,
            width: 300,
            height: 300,
        };
        assert!(bounds.contains(-500.0, 40.0));
        assert!(bounds.contains(-200.1, 339.9));
        assert!(!bounds.contains(-200.0, 100.0));
        assert!(!bounds.contains(-501.0, 100.0));
    }

    #[test]
    fn placement_tracks_the_display_with_the_largest_overlap() {
        let displays = [
            display("primary", 0, 0, 1920, 1040, 1.0),
            display("upper-left", -1280, -1024, 1280, 984, 1.25),
        ];
        let mut settings = RuntimeSettings::default();
        settings.x = -900;
        settings.y = -700;
        settings.width = 300;
        settings.height = 300;

        assert!(settings.capture_display_placement(&displays));
        assert_eq!(settings.monitor.as_deref(), Some("upper-left"));
        assert_eq!(settings.scale_factor, 1.25);
        assert!(settings.relative_x.is_some());
        assert!(settings.relative_y.is_some());
    }

    #[test]
    fn physical_and_logical_dpi_conversions_do_not_accumulate_drift() {
        for scale in [1.0, 1.25, 1.5] {
            let logical = physical_to_logical(360, scale);
            assert_eq!(logical_to_physical(logical, scale), 360);
        }
        assert_eq!(physical_to_logical(360, 1.25), 288.0);
        assert_eq!(physical_to_logical(360, 1.5), 240.0);
    }

    #[test]
    fn removed_monitor_recovers_to_primary_work_area() {
        let mut settings = RuntimeSettings::default();
        settings.monitor = Some("removed".into());
        settings.x = -1200;
        settings.y = 200;
        settings.relative_x = Some(0.8);
        settings.relative_y = Some(0.5);
        let displays = [display("primary", 0, 0, 1920, 1040, 1.0)];

        assert!(settings.restore_display_placement(&displays, Some("primary")));
        assert_eq!((settings.x, settings.y), (40, 40));
        assert_eq!(settings.monitor.as_deref(), Some("primary"));
        assert_eq!(settings.width, settings.height);
    }

    #[test]
    fn resolution_reduction_preserves_relative_placement_and_aspect_ratio() {
        let mut settings = RuntimeSettings::default();
        settings.monitor = Some("primary".into());
        settings.x = 1560;
        settings.y = 680;
        settings.relative_x = Some(1.0);
        settings.relative_y = Some(1.0);
        let displays = [display("primary", 0, 0, 1280, 680, 1.5)];

        settings.restore_display_placement(&displays, Some("primary"));
        assert_eq!((settings.x, settings.y), (920, 320));
        assert_eq!((settings.width, settings.height), (360, 360));
        assert_eq!(settings.scale_factor, 1.5);
    }

    #[test]
    fn partial_and_complete_offscreen_positions_recover_safely() {
        let displays = [display("primary", 0, 0, 1280, 680, 1.0)];
        let mut partial = RuntimeSettings::default();
        partial.x = 1200;
        partial.y = 620;
        partial.restore_display_placement(&displays, Some("primary"));
        assert_eq!((partial.x, partial.y), (920, 320));

        let mut missing = RuntimeSettings::default();
        missing.x = 5000;
        missing.y = -5000;
        missing.restore_display_placement(&displays, Some("primary"));
        assert_eq!((missing.x, missing.y), (40, 40));
    }

    #[test]
    fn physical_cursor_hit_testing_stays_aligned_at_mixed_dpi() {
        let bounds = Bounds {
            x: -300,
            y: 150,
            width: 360,
            height: 360,
        };
        let cursor_x = -300.0 + logical_to_physical(120.0, 1.5) as f64;
        let cursor_y = 150.0 + logical_to_physical(80.0, 1.25) as f64;
        assert!(bounds.contains(cursor_x, cursor_y));
        assert!(!bounds.contains(bounds.x as f64 + bounds.width as f64, cursor_y));
    }

    #[test]
    fn ghost_stays_click_through_until_cursor_leaves() {
        let mut engine = BehaviourEngine::default();
        assert_eq!(tick(&mut engine, true, 1).state, InteractionState::Hiding);
        assert_eq!(
            tick(&mut engine, true, 140).state,
            InteractionState::Ghosted
        );
        assert!(tick(&mut engine, true, 500).click_through);
        assert_eq!(
            tick(&mut engine, false, 1).state,
            InteractionState::ReturnDelay
        );
        assert_eq!(
            tick(&mut engine, false, 150).state,
            InteractionState::Showing
        );
        assert_eq!(
            tick(&mut engine, false, 190).state,
            InteractionState::Visible
        );
    }

    #[test]
    fn enter_during_show_and_exit_during_hide_reverse_safely() {
        let mut engine = BehaviourEngine::default();
        tick(&mut engine, true, 1);
        assert_eq!(
            tick(&mut engine, false, 30).state,
            InteractionState::Showing
        );
        assert_eq!(tick(&mut engine, true, 30).state, InteractionState::Hiding);
    }

    #[test]
    fn edit_overrides_ghost_and_restores_interaction() {
        let mut engine = BehaviourEngine::default();
        tick(&mut engine, true, 1);
        tick(&mut engine, true, 140);
        let output = engine.step(BehaviourInput {
            mode: BehaviourMode::Ghost,
            inside: true,
            editing: true,
            dt: Duration::ZERO,
            hide_delay: Duration::ZERO,
            return_delay: Duration::ZERO,
            fade_opacity: 0.15,
        });
        assert_eq!(output.state, InteractionState::Editing);
        assert!(!output.click_through);
        assert_eq!(output.opacity, 1.0);
    }

    #[test]
    fn invalid_settings_recover_and_preserve_square_geometry() {
        let mut invalid = RuntimeSettings::default();
        invalid.selected_face = "missing".into();
        invalid.width = 4;
        invalid.height = 9999;
        invalid.fade_opacity = f64::NAN;
        invalid.scale_factor = -1.0;
        let invalid = invalid.validated();
        assert_eq!(invalid.selected_face, "koi");
        assert_eq!(invalid.width, MIN_SIZE);
        assert_eq!(invalid.height, MIN_SIZE);
        assert_eq!(invalid.fade_opacity, 0.15);
        assert_eq!(invalid.scale_factor, 1.0);
    }

    #[test]
    fn photo_preserves_source_aspect_ratio() {
        let mut photo = PhotoSettings::new("photo.webp".into(), "image/webp".into(), 1600, 900);
        assert_eq!(photo.object_type, ObjectType::Photo);
        assert_eq!(photo.width, 420);
        assert_eq!(photo.height, 236);
        assert!((photo.aspect_ratio() - 16.0 / 9.0).abs() < f64::EPSILON);
        photo.width = 720;
        photo.height = 405;
        photo.restore_display_placement(&[display("small", 0, 0, 500, 300, 1.0)], Some("small"));
        assert_eq!((photo.width, photo.height), (500, 281));
    }
}
