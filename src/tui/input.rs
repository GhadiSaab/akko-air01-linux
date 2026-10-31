use super::{
    Intent, Mode, UiState, picker_items, raw_action,
    view::{self, UiButton, UiLayout},
};
use crate::{
    device::Layer,
    keyboard::{self, Direction},
};
use crossterm::event::{
    KeyCode, KeyEvent, KeyEventKind, KeyModifiers, MouseButton, MouseEvent, MouseEventKind,
};
use ratatui::layout::{Position, Rect};
impl UiState {
    fn choose_assignment(&mut self, action: crate::protocol::Action) -> Intent {
        let current = self.current_action();
        let action = if current.label() == action.label() {
            current
        } else {
            action
        };
        self.mode = Mode::Browse;
        Intent::Remap(raw_action(action))
    }
    pub fn handle(&mut self, key: KeyEvent) -> Intent {
        if key.kind == KeyEventKind::Release {
            return Intent::None;
        }
        if key.modifiers.contains(KeyModifiers::CONTROL) && key.code == KeyCode::Char('c') {
            return Intent::Quit;
        }
        if matches!(self.mode, Mode::Help | Mode::Notice(_)) {
            self.mode = Mode::Browse;
            return Intent::None;
        }
        let editing_light =
            self.is_lighting() && self.lighting.as_ref().is_some_and(|l| l.editor.is_some());
        if matches!(self.mode, Mode::Browse) && !editing_light {
            match key.code {
                KeyCode::Char('1') => {
                    self.lighting_workspace = false;
                    return Intent::None;
                }
                KeyCode::Char('2') => {
                    self.lighting_workspace = true;
                    return Intent::None;
                }
                KeyCode::F(1) | KeyCode::Char('?') => {
                    self.mode = Mode::Help;
                    return Intent::None;
                }
                _ => {}
            }
        }
        if matches!(self.mode, Mode::Browse) && self.is_lighting() {
            return if let Some(l) = &mut self.lighting {
                l.handle(key, &mut self.status)
            } else {
                match key.code {
                    KeyCode::Char('r') => Intent::ReloadLighting,
                    KeyCode::Char('q') | KeyCode::Esc => Intent::Quit,
                    _ => Intent::None,
                }
            };
        }
        match self.mode.clone() {
            Mode::Picker {
                mut query,
                mut selected,
            } => {
                let count = picker_items(&query, self.layer).len();
                match key.code {
                    KeyCode::Esc => {
                        self.mode = Mode::Browse;
                        return Intent::None;
                    }
                    KeyCode::F(2) => {
                        self.mode = Mode::Edit(raw_action(self.current_action()));
                        return Intent::None;
                    }
                    KeyCode::Enter => {
                        if let Some(item) = picker_items(&query, self.layer).get(selected) {
                            return self.choose_assignment(item.action);
                        }
                    }
                    KeyCode::Up => {
                        if count > 0 {
                            selected = (selected + count - 1) % count;
                        }
                    }
                    KeyCode::Down => {
                        if count > 0 {
                            selected = (selected + 1) % count;
                        }
                    }
                    KeyCode::Backspace => {
                        query.pop();
                        selected = 0;
                    }
                    KeyCode::Char('u') if key.modifiers.contains(KeyModifiers::CONTROL) => {
                        query.clear();
                        selected = 0;
                    }
                    KeyCode::Char(c) if !key.modifiers.contains(KeyModifiers::CONTROL) => {
                        query.push(c);
                        selected = 0;
                    }
                    _ => {}
                }
                self.mode = Mode::Picker { query, selected };
                Intent::None
            }
            Mode::Confirm(_) => match key.code {
                KeyCode::Char('y') | KeyCode::Char('Y') => {
                    self.mode = Mode::Browse;
                    Intent::Confirm(true)
                }
                KeyCode::Char('n') | KeyCode::Esc => {
                    self.mode = Mode::Browse;
                    Intent::Confirm(false)
                }
                _ => Intent::None,
            },
            Mode::Edit(mut value) | Mode::Restore(mut value) => {
                let restore = matches!(self.mode, Mode::Restore(_));
                match key.code {
                    KeyCode::Esc => {
                        self.mode = Mode::Browse;
                        return Intent::None;
                    }
                    KeyCode::Enter => {
                        self.mode = Mode::Browse;
                        return if restore {
                            Intent::Restore(value)
                        } else {
                            Intent::Remap(value)
                        };
                    }
                    KeyCode::Backspace => {
                        value.pop();
                    }
                    KeyCode::Char('u') if key.modifiers.contains(KeyModifiers::CONTROL) => {
                        value.clear()
                    }
                    KeyCode::Char(c) if !key.modifiers.contains(KeyModifiers::CONTROL) => {
                        value.push(c)
                    }
                    _ => {}
                }
                self.mode = if restore {
                    Mode::Restore(value)
                } else {
                    Mode::Edit(value)
                };
                Intent::None
            }
            Mode::Help | Mode::Notice(_) => Intent::None,
            Mode::Browse => match key.code {
                KeyCode::Char('q') | KeyCode::Esc => Intent::Quit,
                KeyCode::Up | KeyCode::Char('k') => {
                    self.selected_slot = keyboard::neighbor(self.selected_slot, Direction::Up);
                    Intent::None
                }
                KeyCode::Down | KeyCode::Char('j') => {
                    self.selected_slot = keyboard::neighbor(self.selected_slot, Direction::Down);
                    Intent::None
                }
                KeyCode::Left | KeyCode::Char('h') => {
                    self.selected_slot = keyboard::neighbor(self.selected_slot, Direction::Left);
                    Intent::None
                }
                KeyCode::Right | KeyCode::Char('l') => {
                    self.selected_slot = keyboard::neighbor(self.selected_slot, Direction::Right);
                    Intent::None
                }
                KeyCode::PageUp | KeyCode::Char('[') => {
                    self.selected_profile = (self.selected_profile + 3) % 4;
                    Intent::None
                }
                KeyCode::PageDown | KeyCode::Char(']') => {
                    self.selected_profile = (self.selected_profile + 1) % 4;
                    Intent::None
                }
                KeyCode::Tab => {
                    self.layer = match self.layer {
                        Layer::Base => Layer::FnWin,
                        Layer::FnWin => Layer::FnMac,
                        Layer::FnMac => Layer::Base,
                    };
                    Intent::None
                }
                KeyCode::Char('e') | KeyCode::Enter => {
                    self.open_picker();
                    Intent::None
                }
                KeyCode::Char('r') => {
                    self.mode = Mode::Restore(String::new());
                    Intent::None
                }
                KeyCode::Char('s') => Intent::Swap,
                KeyCode::Char('b') => Intent::Backup,
                KeyCode::Char('a') => Intent::Activate,
                _ => Intent::None,
            },
        }
    }
    pub fn handle_mouse(&mut self, event: MouseEvent, area: Rect) -> Intent {
        if area.width < 80 || area.height < 24 {
            return Intent::None;
        }
        if matches!(self.mode, Mode::Help | Mode::Notice(_)) {
            if event.kind == MouseEventKind::Down(MouseButton::Left) {
                self.mode = Mode::Browse;
            }
            return Intent::None;
        }
        let editing_light =
            self.is_lighting() && self.lighting.as_ref().is_some_and(|l| l.editor.is_some());
        if matches!(self.mode, Mode::Browse)
            && !editing_light
            && event.kind == MouseEventKind::Down(MouseButton::Left)
        {
            let [keys, lighting, help] = super::theme::navigation(area);
            let point = Position::new(event.column, event.row);
            if keys.contains(point) {
                self.lighting_workspace = false;
                return Intent::None;
            }
            if lighting.contains(point) {
                self.lighting_workspace = true;
                return Intent::None;
            }
            if help.contains(point) {
                self.mode = Mode::Help;
                return Intent::None;
            }
        }
        if matches!(self.mode, Mode::Browse) && self.is_lighting() {
            if let Some(l) = &mut self.lighting {
                return l.mouse(event, area, &mut self.status);
            }
            let reload = super::lighting::LightLayout::new(area)
                .controls
                .into_iter()
                .find(|(i, _)| *i == 13)
                .expect("reload control")
                .1;
            return if event.kind == MouseEventKind::Down(MouseButton::Left)
                && reload.contains(Position::new(event.column, event.row))
            {
                Intent::ReloadLighting
            } else {
                Intent::None
            };
        }
        if matches!(self.mode, Mode::Picker { .. })
            && matches!(
                event.kind,
                MouseEventKind::ScrollUp | MouseEventKind::ScrollDown
            )
        {
            return self.handle(KeyEvent::new(
                if event.kind == MouseEventKind::ScrollUp {
                    KeyCode::Up
                } else {
                    KeyCode::Down
                },
                KeyModifiers::NONE,
            ));
        }
        if event.kind != MouseEventKind::Down(MouseButton::Left) {
            return Intent::None;
        }
        let point = Position::new(event.column, event.row);
        match self.mode.clone() {
            Mode::Picker { query, selected } => {
                let popup = view::picker_popup(area);
                if !popup.contains(point) {
                    self.mode = Mode::Browse;
                    return Intent::None;
                }
                let list = view::picker_list(area);
                if list.contains(point) {
                    let start = view::picker_start(selected, list.height);
                    let index = start + (event.row - list.y) as usize;
                    if let Some(item) = picker_items(&query, self.layer).get(index) {
                        return self.choose_assignment(item.action);
                    }
                }
                Intent::None
            }
            Mode::Confirm(_) => {
                let (apply, cancel) = view::confirm_buttons(area);
                if apply.contains(point) {
                    self.mode = Mode::Browse;
                    Intent::Confirm(true)
                } else if cancel.contains(point) || !view::confirm_popup(area).contains(point) {
                    self.mode = Mode::Browse;
                    Intent::Confirm(false)
                } else {
                    Intent::None
                }
            }
            Mode::Help | Mode::Notice(_) | Mode::Edit(_) | Mode::Restore(_) => Intent::None,
            Mode::Browse => {
                let layout = UiLayout::new(area);
                for (profile, r) in &layout.profile_tabs {
                    if r.contains(point) {
                        self.selected_profile = *profile;
                        return Intent::None;
                    }
                }
                for (layer, r) in &layout.layer_tabs {
                    if r.contains(point) {
                        self.layer = *layer;
                        return Intent::None;
                    }
                }
                if layout.activate.contains(point) {
                    return Intent::Activate;
                }
                for (button, r) in &layout.buttons {
                    if r.contains(point) {
                        return match button {
                            UiButton::Backup => Intent::Backup,
                            UiButton::Restore => {
                                self.mode = Mode::Restore(String::new());
                                Intent::None
                            }
                            UiButton::Swap => Intent::Swap,
                            UiButton::Quit => Intent::Quit,
                        };
                    }
                }
                if let Some(slot) =
                    keyboard::hit(layout.keyboard, layout.large, event.column, event.row)
                {
                    self.selected_slot = slot;
                    self.open_picker();
                }
                Intent::None
            }
        }
    }
}
