use serde::{Deserialize, Serialize};
use std::time::Duration;

pub const MIN_SIZE: u32 = 180;
pub const MAX_SIZE: u32 = 720;

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
#[serde(rename_all = "camelCase")]
pub struct RuntimeSettings {
    pub selected_face: String,
    pub x: i32,
    pub y: i32,
    pub width: u32,
    pub height: u32,
    pub monitor: Option<String>,
    pub scale_factor: f64,
    pub always_on_top: bool,
    pub locked: bool,
    pub behaviour: BehaviourMode,
    pub ghost_hide_delay: u64,
    pub ghost_return_delay: u64,
    pub fade_opacity: f64,
    pub show_second_hand: bool,
    pub smooth_movement: bool,
    pub visible: bool,
    pub launch_at_login: bool,
}

impl Default for RuntimeSettings {
    fn default() -> Self {
        Self {
            selected_face: "koi".into(),
            x: 80,
            y: 80,
            width: 360,
            height: 360,
            monitor: None,
            scale_factor: 1.0,
            always_on_top: true,
            locked: true,
            behaviour: BehaviourMode::Ghost,
            ghost_hide_delay: 0,
            ghost_return_delay: 150,
            fade_opacity: 0.15,
            show_second_hand: true,
            smooth_movement: true,
            visible: true,
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
        self.width = self.width.clamp(MIN_SIZE, MAX_SIZE);
        self.height = self.width;
        self.fade_opacity = if self.fade_opacity.is_finite() {
            self.fade_opacity.clamp(0.05, 0.5)
        } else {
            defaults.fade_opacity
        };
        self.scale_factor = if self.scale_factor.is_finite() && self.scale_factor > 0.0 {
            self.scale_factor
        } else {
            1.0
        };
        self.ghost_hide_delay = self.ghost_hide_delay.min(2_000);
        self.ghost_return_delay = self.ghost_return_delay.min(2_000);
        self
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
        let invalid = RuntimeSettings {
            selected_face: "missing".into(),
            width: 4,
            height: 9999,
            fade_opacity: f64::NAN,
            scale_factor: -1.0,
            ..RuntimeSettings::default()
        }
        .validated();
        assert_eq!(invalid.selected_face, "koi");
        assert_eq!(invalid.width, MIN_SIZE);
        assert_eq!(invalid.height, MIN_SIZE);
        assert_eq!(invalid.fade_opacity, 0.15);
        assert_eq!(invalid.scale_factor, 1.0);
    }
}
