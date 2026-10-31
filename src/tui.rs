//! Visual terminal frontend using the existing verified transaction backend.
use crate::{
    device::{Board, Layer},
    operations::{self, Change},
    protocol::Action,
    snapshot::Snapshot,
    transport::Wire,
};
use anyhow::Result;
use crossterm::{
    ExecutableCommand,
    event::{self, DisableMouseCapture, EnableMouseCapture, Event, KeyCode, KeyModifiers},
    terminal::{EnterAlternateScreen, LeaveAlternateScreen, disable_raw_mode, enable_raw_mode},
};
use ratatui::prelude::*;
use std::{io, path::Path, time::Duration};
mod input;
mod view;
pub use view::{UiLayout, draw};
#[derive(Clone, Debug, PartialEq, Eq)]
pub enum Mode {
    Browse,
    Picker { query: String, selected: usize },
    Edit(String),
    Restore(String),
    Confirm(String),
}
#[derive(Debug, PartialEq, Eq)]
pub enum Intent {
    None,
    Quit,
    Swap,
    Backup,
    Activate,
    Remap(String),
    Restore(String),
    Confirm(bool),
}
pub struct UiState {
    pub snapshot: Snapshot,
    pub selected_profile: u8,
    pub selected_slot: u8,
    pub layer: Layer,
    pub mode: Mode,
    pub status: String,
}
impl UiState {
    pub fn new(snapshot: Snapshot) -> Self {
        Self{selected_profile:snapshot.active_profile,snapshot,selected_slot:0,layer:Layer::Base,mode:Mode::Browse,status:"Click a key to change its assignment. Profile tabs preview; Activate switches the board.".into()}
    }
    pub fn confirm(&mut self, s: String) {
        self.mode = Mode::Confirm(s);
    }
    pub fn current_action(&self) -> Action {
        self.snapshot
            .profile(self.selected_profile)
            .map(|p| operations::entries(p, self.layer)[self.selected_slot as usize])
            .unwrap_or(Action::DISABLED)
    }
    pub fn open_picker(&mut self) {
        let action = self.current_action();
        let selected = picker_items("", self.layer)
            .iter()
            .position(|p| p.action == action || p.action.label() == action.label())
            .unwrap_or(0);
        self.mode = Mode::Picker {
            query: String::new(),
            selected,
        };
    }
}
#[derive(Clone)]
pub(crate) struct PickerItem {
    pub label: String,
    pub group: &'static str,
    pub action: Action,
}
pub(crate) fn picker_items(query: &str, layer: Layer) -> Vec<PickerItem> {
    let mut items = vec![
        PickerItem {
            label: "Fn".into(),
            group: "Keyboard",
            action: Action::FN,
        },
        PickerItem {
            label: if layer == Layer::Base {
                "Disabled"
            } else {
                "Inherit base"
            }
            .into(),
            group: "Keyboard",
            action: Action::DISABLED,
        },
        PickerItem {
            label: "Toggle lighting".into(),
            group: "Keyboard",
            action: Action([10, 9, 0, 0]),
        },
    ];
    for code in (224..=231).chain(4..=82) {
        let label = crate::protocol::usage_label(code);
        if label.starts_with("HID:") {
            continue;
        }
        items.push(PickerItem {
            label,
            group: if code >= 224 {
                "Modifier"
            } else if (58..=69).contains(&code) {
                "Function"
            } else {
                "Key"
            },
            action: Action([0, 0, code, 0]),
        });
    }
    let query = query.to_lowercase();
    items
        .into_iter()
        .filter(|p| p.label.to_lowercase().contains(&query))
        .collect()
}
pub(crate) fn raw_action(a: Action) -> String {
    format!(
        "raw:{:02x}:{:02x}:{:02x}:{:02x}",
        a.0[0], a.0[1], a.0[2], a.0[3]
    )
}
struct TerminalGuard;
impl Drop for TerminalGuard {
    fn drop(&mut self) {
        let _ = io::stdout().execute(DisableMouseCapture);
        let _ = disable_raw_mode();
        let _ = io::stdout().execute(LeaveAlternateScreen);
        let _ = io::stdout().execute(crossterm::cursor::Show);
    }
}
enum PendingKind {
    Keys(Vec<Change>),
    Profile(u8),
}
struct Pending {
    snapshot: Snapshot,
    kind: PendingKind,
}
pub fn run<W: Wire>(board: &mut Board<W>) -> Result<()> {
    eprintln!("Reading all profiles and Fn maps…");
    let mut ui = UiState::new(board.snapshot()?);
    let mut pending: Option<Pending> = None;
    enable_raw_mode()?;
    let _guard = TerminalGuard;
    io::stdout()
        .execute(EnterAlternateScreen)?
        .execute(EnableMouseCapture)?;
    let mut terminal = Terminal::new(CrosstermBackend::new(io::stdout()))?;
    terminal.clear()?;
    loop {
        terminal.draw(|f| draw(f, &ui))?;
        if !event::poll(Duration::from_millis(200))? {
            continue;
        }
        let size = terminal.size()?;
        let area = Rect::new(0, 0, size.width, size.height);
        let intent = match event::read()? {
            Event::Key(key) => {
                if area.width < 80 || area.height < 24 {
                    if key.code == KeyCode::Char('q')
                        || key.code == KeyCode::Esc
                        || (key.code == KeyCode::Char('c')
                            && key.modifiers.contains(KeyModifiers::CONTROL))
                    {
                        Intent::Quit
                    } else {
                        Intent::None
                    }
                } else {
                    ui.handle(key)
                }
            }
            Event::Mouse(mouse) => ui.handle_mouse(mouse, area),
            _ => Intent::None,
        };
        if intent == Intent::Quit {
            break;
        }
        let result = (|| -> Result<()> {
            match intent {
                Intent::None | Intent::Quit => {}
                Intent::Backup => {
                    ui.status = "Reading current settings for backup…".into();
                    terminal.draw(|f| draw(f, &ui))?;
                    let fresh = board.snapshot()?;
                    let path = operations::auto_backup(&fresh)?;
                    ui.snapshot = fresh;
                    ui.status = format!("Backup saved: {}", path.display());
                }
                Intent::Swap | Intent::Remap(_) | Intent::Restore(_) => {
                    let changes = match &intent {
                        Intent::Swap => {
                            operations::plan_swap(&ui.snapshot, &[ui.selected_profile])?
                        }
                        Intent::Remap(value) => operations::plan_remap(
                            &ui.snapshot,
                            ui.selected_profile,
                            ui.layer,
                            ui.selected_slot,
                            value.parse::<Action>()?,
                        )?,
                        Intent::Restore(path) => operations::plan_restore(
                            &ui.snapshot,
                            &operations::load_snapshot(Path::new(path.trim()))?,
                        )?,
                        _ => unreachable!(),
                    };
                    if changes.is_empty() {
                        ui.status = "Already configured; no writes needed.".into();
                    } else {
                        let description = changes
                            .iter()
                            .take(4)
                            .map(|c| {
                                format!(
                                    "p{} {} slot {}: {} → {}",
                                    c.profile,
                                    c.layer,
                                    c.slot,
                                    c.before.label(),
                                    c.after.label()
                                )
                            })
                            .collect::<Vec<_>>()
                            .join("; ");
                        ui.confirm(format!("{} changes: {description}", changes.len()));
                        pending = Some(Pending {
                            snapshot: ui.snapshot.clone(),
                            kind: PendingKind::Keys(changes),
                        });
                    }
                }
                Intent::Activate => {
                    if ui.selected_profile != ui.snapshot.active_profile {
                        ui.confirm(format!(
                            "Activate profile {} (currently {})",
                            ui.selected_profile, ui.snapshot.active_profile
                        ));
                        pending = Some(Pending {
                            snapshot: ui.snapshot.clone(),
                            kind: PendingKind::Profile(ui.selected_profile),
                        });
                    }
                }
                Intent::Confirm(false) => {
                    pending = None;
                    ui.status = "Cancelled; no writes sent.".into();
                }
                Intent::Confirm(true) => {
                    if let Some(p) = pending.take() {
                        ui.status = "Saving backup and applying verified changes…".into();
                        terminal.draw(|f| draw(f, &ui))?;
                        let planned_changes = match &p.kind {
                            PendingKind::Keys(changes) => changes.as_slice(),
                            PendingKind::Profile(_) => &[],
                        };
                        let fresh =
                            operations::fresh_for_plan(board, &p.snapshot, planned_changes)?;
                        let backup = operations::auto_backup(&fresh)?;
                        let operation = match p.kind {
                            PendingKind::Keys(changes) => {
                                operations::apply(board, &fresh, &changes, false).map(|_| ())
                            }
                            PendingKind::Profile(pr) => {
                                anyhow::ensure!(
                                    board.active_profile()? == p.snapshot.active_profile,
                                    "profile changed externally; select it again"
                                );
                                operations::select_profile(board, pr)
                            }
                        };
                        let status = match operation {
                            Ok(()) => format!("Read-back verified. Backup: {}", backup.display()),
                            Err(e) => format!("Error: {e:#}. Backup: {}", backup.display()),
                        };
                        match board.snapshot() {
                            Ok(s) => {
                                ui.snapshot = s;
                                ui.status = status;
                            }
                            Err(e) => {
                                ui.status =
                                    format!("{status}. Refresh failed: {e}. Reconnect and reopen.");
                            }
                        }
                    }
                }
            }
            Ok(())
        })();
        if let Err(e) = result {
            ui.mode = Mode::Browse;
            pending = None;
            ui.status = format!("Error: {e:#}");
        }
    }
    Ok(())
}
