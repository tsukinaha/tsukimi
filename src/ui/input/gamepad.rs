use std::{
    cell::RefCell,
    collections::HashMap,
};

use super::{
    actions::InputAction,
    gamepad_profile::GamepadProfile,
};
use crate::{
    Window,
    ui::widgets::utils::GlobalToast,
};
use gilrs::{
    Axis,
    Button,
    EventType,
    GamepadId,
    Gilrs,
    GilrsBuilder,
};

pub struct GamepadManager {
    gilrs: RefCell<Option<Gilrs>>,
    active_profile: RefCell<GamepadProfile>,
    active_name: RefCell<String>,
    primary_id: RefCell<Option<GamepadId>>,
    button_active: RefCell<HashMap<(GamepadId, Button), bool>>,
    axis_active: RefCell<HashMap<(GamepadId, Axis), bool>>,
    stick_deadzone: f32,
}

impl Default for GamepadManager {
    fn default() -> Self {
        Self {
            gilrs: RefCell::new(open_gilrs()),
            active_profile: RefCell::new(GamepadProfile::Generic),
            active_name: RefCell::new(String::new()),
            primary_id: RefCell::new(None),
            button_active: RefCell::new(HashMap::new()),
            axis_active: RefCell::new(HashMap::new()),
            stick_deadzone: 0.15,
        }
    }
}

impl GamepadManager {
    pub fn has_connected_controller(&self) -> bool {
        self.primary_id.borrow().is_some()
    }

    pub fn poll(&mut self, window: &Window) -> Vec<InputAction> {
        let mut actions = Vec::new();
        let mut recreate = false;

        {
            let mut gilrs_guard = self.gilrs.borrow_mut();
            if gilrs_guard.is_none() {
                *gilrs_guard = open_gilrs();
            }
            let Some(gilrs) = gilrs_guard.as_mut() else {
                return actions;
            };

            self.refresh_primary_gamepad(gilrs);

            while let Some(event) = gilrs.next_event() {
                let gamepad = gilrs.gamepad(event.id);
                let name = gamepad.name().to_string();

                if !is_game_controller(&name) {
                    continue;
                }

                match event.event {
                    EventType::Connected => {
                        self.clear_gamepad_state(event.id);
                        self.refresh_primary_gamepad(gilrs);
                        *self.active_name.borrow_mut() = name.clone();
                        *self.active_profile.borrow_mut() = GamepadProfile::detect(&name);
                        window.toast(format!("{} connected", name));
                        continue;
                    }
                    EventType::Disconnected => {
                        self.clear_gamepad_state(event.id);
                        *self.primary_id.borrow_mut() = None;
                        self.active_name.borrow_mut().clear();
                        window.toast(format!("{} disconnected", name));
                        recreate = true;
                        break;
                    }
                    _ => {}
                }

                self.refresh_primary_gamepad(gilrs);
                if self.primary_id.borrow().is_some_and(|id| id != event.id) {
                    continue;
                }

                if let Some(action) = self.map_event(event.id, event.event) {
                    actions.push(action);
                }
            }

            self.poll_cached_state(gilrs, &mut actions);
        }

        if recreate {
            self.reset_after_disconnect();
        }

        actions
    }

    pub fn active_profile(&self) -> GamepadProfile {
        *self.active_profile.borrow()
    }

    pub fn active_name(&self) -> String {
        self.active_name.borrow().clone()
    }

    fn refresh_primary_gamepad(&self, gilrs: &Gilrs) {
        let best = gilrs
            .gamepads()
            .filter(|(_, gamepad)| is_game_controller(gamepad.name()))
            .max_by_key(|(_, gamepad)| gamepad_priority(gamepad.name()))
            .map(|(id, _)| id);
        *self.primary_id.borrow_mut() = best;
        if let Some(id) = best {
            self.track_active_gamepad(id, gilrs);
        }
    }

    fn track_active_gamepad(&self, id: GamepadId, gilrs: &Gilrs) {
        let name = gilrs.gamepad(id).name().to_string();
        *self.active_name.borrow_mut() = name.clone();
        *self.active_profile.borrow_mut() = GamepadProfile::detect(&name);
    }

    fn clear_gamepad_state(&self, id: GamepadId) {
        self.button_active
            .borrow_mut()
            .retain(|(gamepad_id, _), _| *gamepad_id != id);
        self.axis_active
            .borrow_mut()
            .retain(|(gamepad_id, _), _| *gamepad_id != id);
    }

    fn reset_after_disconnect(&self) {
        self.button_active.borrow_mut().clear();
        self.axis_active.borrow_mut().clear();
        *self.primary_id.borrow_mut() = None;
        *self.gilrs.borrow_mut() = open_gilrs();
    }

    fn poll_cached_state(&self, gilrs: &Gilrs, actions: &mut Vec<InputAction>) {
        let Some(id) = *self.primary_id.borrow() else {
            return;
        };
        let gamepad = gilrs.gamepad(id);
        if !gamepad.is_connected() {
            return;
        }

        let button_map = [
            (Button::South, InputAction::Activate),
            (Button::East, InputAction::Back),
            (Button::West, InputAction::Search),
            (Button::Start, InputAction::Menu),
            (Button::Select, InputAction::ToggleHints),
            (Button::Mode, InputAction::SwitchGamepad),
            (Button::DPadUp, InputAction::NavigateUp),
            (Button::DPadDown, InputAction::NavigateDown),
            (Button::DPadLeft, InputAction::NavigateLeft),
            (Button::DPadRight, InputAction::NavigateRight),
            (Button::LeftTrigger, InputAction::PageScrollLeft),
            (Button::RightTrigger, InputAction::PageScrollRight),
            (Button::LeftTrigger2, InputAction::PageScrollLeft),
            (Button::RightTrigger2, InputAction::PageScrollRight),
        ];

        for (button, action) in button_map {
            if self.consume_button_edge(id, button, gamepad.is_pressed(button)) {
                crate::tv::osk::mark_gamepad_input();
                actions.push(action);
            }
        }

        self.poll_stick_axis(id, &gamepad, Axis::LeftStickX, actions);
        self.poll_stick_axis(id, &gamepad, Axis::LeftStickY, actions);
    }

    fn poll_stick_axis(
        &self, id: GamepadId, gamepad: &gilrs::Gamepad<'_>, axis: Axis,
        actions: &mut Vec<InputAction>,
    ) {
        let value = gamepad.value(axis);
        let active = value.abs() >= self.stick_deadzone;
        if !self.consume_axis_edge(id, axis, active) {
            return;
        }
        crate::tv::osk::mark_gamepad_input();
        let action = match axis {
            Axis::LeftStickX if value > 0.0 => InputAction::NavigateRight,
            Axis::LeftStickX => InputAction::NavigateLeft,
            Axis::LeftStickY if value < 0.0 => InputAction::NavigateUp,
            Axis::LeftStickY => InputAction::NavigateDown,
            _ => return,
        };
        actions.push(action);
    }

    fn consume_button_edge(&self, id: GamepadId, button: Button, pressed: bool) -> bool {
        let key = (id, button);
        let was_pressed = self
            .button_active
            .borrow()
            .get(&key)
            .copied()
            .unwrap_or(false);
        self.button_active.borrow_mut().insert(key, pressed);
        pressed && !was_pressed
    }

    fn consume_axis_edge(&self, id: GamepadId, axis: Axis, active: bool) -> bool {
        let key = (id, axis);
        let was_active = self
            .axis_active
            .borrow()
            .get(&key)
            .copied()
            .unwrap_or(false);
        self.axis_active.borrow_mut().insert(key, active);
        active && !was_active
    }

    fn map_event(&self, id: GamepadId, event: EventType) -> Option<InputAction> {
        use InputAction::*;
        match event {
            EventType::ButtonPressed(button, _) => {
                self.button_active.borrow_mut().insert((id, button), true);
                self.button_action(button)
            }
            EventType::ButtonRepeated(button, _) => self.button_action(button),
            EventType::ButtonChanged(button, value, _) if value > 0.5 => {
                if self.consume_button_edge(id, button, true) {
                    self.button_action(button)
                } else {
                    None
                }
            }
            EventType::ButtonChanged(button, value, _) if value <= 0.5 => {
                self.button_active.borrow_mut().insert((id, button), false);
                None
            }
            EventType::AxisChanged(axis, value, _) => {
                let active = value.abs() >= self.stick_deadzone;
                if !self.consume_axis_edge(id, axis, active) {
                    return None;
                }
                match axis {
                    Axis::LeftStickX if value > 0.0 => Some(NavigateLeft),
                    Axis::LeftStickX => Some(NavigateRight),
                    Axis::LeftStickY if value < 0.0 => Some(NavigateDown),
                    Axis::LeftStickY => Some(NavigateUp),
                    _ => None,
                }
            }
            _ => None,
        }
    }

    fn button_action(&self, button: Button) -> Option<InputAction> {
        use InputAction::*;
        match button {
            Button::South => Some(Activate),
            Button::East => Some(Back),
            Button::West => Some(Search),
            Button::Start => Some(Menu),
            Button::Select => Some(ToggleHints),
            Button::Mode => Some(SwitchGamepad),
            Button::DPadUp => Some(NavigateUp),
            Button::DPadDown => Some(NavigateDown),
            Button::DPadLeft => Some(NavigateLeft),
            Button::DPadRight => Some(NavigateRight),
            Button::LeftTrigger | Button::LeftTrigger2 => Some(PageScrollLeft),
            Button::RightTrigger | Button::RightTrigger2 => Some(PageScrollRight),
            _ => None,
        }
    }
}

fn open_gilrs() -> Option<Gilrs> {
    GilrsBuilder::new().with_force_feedback(false).build().ok()
}

fn is_game_controller(name: &str) -> bool {
    let lower = name.to_lowercase();
    !lower.contains("stick")
        && !lower.contains("throttle")
        && !lower.contains("rudder")
        && !lower.contains("pedal")
        && !lower.contains("hotas")
        && !lower.contains("flight")
        && !lower.contains("yoke")
}

fn gamepad_priority(name: &str) -> i32 {
    match GamepadProfile::detect(name) {
        GamepadProfile::Xbox
        | GamepadProfile::PlayStation
        | GamepadProfile::SteamDeck
        | GamepadProfile::Nintendo => 100,
        GamepadProfile::Generic => 10,
    }
}
