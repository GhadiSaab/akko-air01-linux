//! Terminal frontend. Navigation never changes a hardware profile. Mutations
//! use the same backup, preflight, and verified transaction code as the CLI.
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
    event::{self, Event, KeyCode, KeyEvent, KeyEventKind, KeyModifiers},
    terminal::{EnterAlternateScreen, LeaveAlternateScreen, disable_raw_mode, enable_raw_mode},
};
use ratatui::{
    prelude::*,
    widgets::{Block, List, ListItem, ListState, Paragraph, Wrap},
};
use std::{io, path::Path, time::Duration};

#[derive(Clone, Debug, PartialEq, Eq)]
pub enum Mode {
    Browse,
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
        Self{selected_profile:snapshot.active_profile,snapshot,selected_slot:0,layer:Layer::Base,mode:Mode::Browse,status:"Navigation previews profiles; press a to activate. All changes require confirmation.".into()}
    }
    pub fn confirm(&mut self, s: String) {
        self.mode = Mode::Confirm(s);
    }
    pub fn handle(&mut self, key: KeyEvent) -> Intent {
        if key.kind == KeyEventKind::Release {
            return Intent::None;
        }
        if key.modifiers.contains(KeyModifiers::CONTROL) && key.code == KeyCode::Char('c') {
            return Intent::Quit;
        }
        match self.mode.clone() {
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
            Mode::Browse => match key.code {
                KeyCode::Char('q') | KeyCode::Esc => Intent::Quit,
                KeyCode::Up | KeyCode::Char('k') => {
                    self.selected_slot = self.selected_slot.wrapping_sub(1) & 127;
                    Intent::None
                }
                KeyCode::Down | KeyCode::Char('j') => {
                    self.selected_slot = (self.selected_slot + 1) & 127;
                    Intent::None
                }
                KeyCode::Left => {
                    self.selected_profile = (self.selected_profile + 3) % 4;
                    Intent::None
                }
                KeyCode::Right => {
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
                    let value = self
                        .snapshot
                        .profile(self.selected_profile)
                        .map(|p| {
                            operations::entries(p, self.layer)[self.selected_slot as usize].label()
                        })
                        .unwrap_or_default();
                    self.mode = Mode::Edit(value);
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
}
pub fn draw(f: &mut Frame, s: &UiState) {
    let chunks = Layout::vertical([
        Constraint::Length(3),
        Constraint::Min(4),
        Constraint::Length(4),
        Constraint::Length(4),
    ])
    .split(f.area());
    let header = format!(
        "Air 01  |  active: {}  |  viewing: {}  |  layer: {}",
        s.snapshot.active_profile, s.selected_profile, s.layer
    );
    f.render_widget(
        Paragraph::new(header)
            .block(Block::bordered().title(" air01 "))
            .style(Style::default().fg(Color::Cyan)),
        chunks[0],
    );
    let items = s
        .snapshot
        .profile(s.selected_profile)
        .map(|p| {
            operations::entries(p, s.layer)
                .iter()
                .enumerate()
                .map(|(i, a)| {
                    let physical = match i {
                        5 => " [physical Fn]",
                        11 => " [physical LeftCtrl]",
                        _ => "",
                    };
                    ListItem::new(format!(
                        "{i:3}{physical:21} {:23} {:02x}:{:02x}:{:02x}:{:02x}",
                        a.label(),
                        a.0[0],
                        a.0[1],
                        a.0[2],
                        a.0[3]
                    ))
                })
                .collect::<Vec<_>>()
        })
        .unwrap_or_default();
    let list = List::new(items)
        .block(Block::bordered().title(" Matrix slots / assigned action / raw bytes "))
        .highlight_style(Style::default().bg(Color::DarkGray).fg(Color::White))
        .highlight_symbol("> ");
    let mut state = ListState::default().with_selected(Some(s.selected_slot as usize));
    f.render_stateful_widget(list, chunks[1], &mut state);
    let prompt = match &s.mode {
        Mode::Browse => s.status.clone(),
        Mode::Edit(v) => format!(
            "Action: {v}_  (key name, Fn, Disabled, raw:00:00:e0:00). Enter: plan, Esc: cancel"
        ),
        Mode::Restore(v) => {
            format!("Restore snapshot absolute path: {v}_  Enter: plan, Esc: cancel")
        }
        Mode::Confirm(v) => format!("{v}\nApply? y = yes; n / Esc = cancel"),
    };
    f.render_widget(
        Paragraph::new(prompt)
            .wrap(Wrap { trim: false })
            .block(Block::bordered().title(" Status ")),
        chunks[2],
    );
    f.render_widget(Paragraph::new("↑/↓ select  ←/→ preview profile  Tab layer  e edit  s Ctrl/Fn swap\na activate profile  b backup  r restore  q quit  Ctrl+C always exits").block(Block::bordered()),chunks[3]);
}
struct TerminalGuard;
impl Drop for TerminalGuard {
    fn drop(&mut self) {
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
    io::stdout().execute(EnterAlternateScreen)?;
    let mut terminal = Terminal::new(CrosstermBackend::new(io::stdout()))?;
    terminal.clear()?;
    loop {
        terminal.draw(|f| draw(f, &ui))?;
        if !event::poll(Duration::from_millis(200))? {
            continue;
        }
        let Event::Key(key) = event::read()? else {
            continue;
        };
        let intent = ui.handle(key);
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
