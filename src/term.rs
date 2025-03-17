use std::io::{BufRead, BufReader};
use std::thread;
use std::time::Duration;

use anyhow::{Context, Result};
use portable_pty::{ChildKiller, CommandBuilder, PtyPair, PtySize, native_pty_system};
use termwiz::{
    cell::AttributeChange,
    escape::{
        Action, CSI, ControlCode,
        csi::{Cursor, Sgr},
        parser::Parser,
    },
    surface::{Change, Position, SEQ_ZERO, SequenceNo, Surface},
};

pub struct Terminal {
    surface: Surface,
    parser: Parser,
    state: State,
    pty: PtyPair,
}

impl Terminal {
    pub fn new(cols: u16, rows: u16) -> Result<Self> {
        // Define terminal size.
        let size = PtySize {
            cols,
            rows,
            pixel_width: 0,
            pixel_height: 0,
        };

        // Create a PTY pair using portable-pty.
        let pty = native_pty_system();
        let pair = pty.openpty(size)?;

        Ok(Self {
            surface: Surface::new(cols.into(), rows.into()),
            parser: Parser::new(),
            state: State::default(),
            pty: pair,
        })
    }

    pub fn surface(&self) -> &Surface {
        &self.surface
    }

    pub fn feed(&mut self, mut reader: impl BufRead) -> Result<()> {
        loop {
            let buffer = reader.fill_buf().context("error reading PTY")?;
            if buffer.is_empty() {
                return Ok(());
            }

            self.parser.parse(buffer, |action| {
                let seq = Self::apply_action(&mut self.surface, &mut self.state, action);
                self.surface.flush_changes_older_than(seq);
            });

            let len = buffer.len();
            reader.consume(len);
        }
    }

    pub fn run(&mut self, mut cmd: CommandBuilder, timeout: Option<Duration>) -> Result<()> {
        cmd.env("TERM", "xterm-256color");
        cmd.env("COLORTERM", "truecolor");
        if cmd.get_cwd().is_none() {
            cmd.cwd(".");
        }

        let reader = BufReader::new(self.pty.master.try_clone_reader()?);
        let _writer = self.pty.master.take_writer()?;
        let mut child = self.pty.slave.spawn_command(cmd)?;
        let killer = child.clone_killer();

        thread::scope(|s| {
            let thread = s.spawn(move || self.feed(reader));

            with_timeout(timeout, killer, s, || child.wait())?;
            thread.join().unwrap()
        })?;

        Ok(())
    }

    fn apply_action(surface: &mut Surface, st: &mut State, action: Action) -> SequenceNo {
        match action {
            Action::Print(ch) => surface.add_change(ch),
            Action::PrintString(s) => surface.add_change(s),
            Action::Control(code) => match code {
                ControlCode::LineFeed => surface.add_change("\r\n"),
                ControlCode::CarriageReturn | ControlCode::HorizontalTab => {
                    surface.add_change(code as u8 as char)
                }
                _ => SEQ_ZERO,
            },
            Action::CSI(csi) => match csi {
                CSI::Sgr(sgr) => match sgr {
                    Sgr::Reset => surface.add_change(Change::AllAttributes(Default::default())),
                    Sgr::Intensity(intensity) => {
                        surface.add_change(Change::Attribute(AttributeChange::Intensity(intensity)))
                    }
                    Sgr::Underline(underline) => {
                        surface.add_change(Change::Attribute(AttributeChange::Underline(underline)))
                    }
                    Sgr::UnderlineColor(_) => SEQ_ZERO,
                    Sgr::Blink(_) => SEQ_ZERO,
                    Sgr::Inverse(inverse) => {
                        surface.add_change(Change::Attribute(AttributeChange::Reverse(inverse)))
                    }
                    Sgr::Foreground(color) => surface
                        .add_change(Change::Attribute(AttributeChange::Foreground(color.into()))),
                    Sgr::Background(color) => surface
                        .add_change(Change::Attribute(AttributeChange::Background(color.into()))),
                    Sgr::Italic(italic) => {
                        surface.add_change(Change::Attribute(AttributeChange::Italic(italic)))
                    }
                    Sgr::StrikeThrough(enabled) => surface
                        .add_change(Change::Attribute(AttributeChange::StrikeThrough(enabled))),
                    Sgr::Invisible(enabled) => {
                        surface.add_change(Change::Attribute(AttributeChange::Invisible(enabled)))
                    }
                    Sgr::Font(_) => SEQ_ZERO,
                    Sgr::VerticalAlign(_) => SEQ_ZERO,
                    Sgr::Overline(_) => SEQ_ZERO,
                },
                CSI::Cursor(cursor) => match cursor {
                    Cursor::BackwardTabulation(n) => surface.add_change(Change::CursorPosition {
                        x: Position::Absolute(tabulate_back(
                            surface.cursor_position().0,
                            n as usize,
                        )),
                        y: Position::Relative(0),
                    }),
                    Cursor::ForwardTabulation(n) => surface.add_change(Change::CursorPosition {
                        x: Position::Absolute(tabulate(surface.cursor_position().0, n as usize)),
                        y: Position::Relative(0),
                    }),
                    Cursor::TabulationClear(_) => SEQ_ZERO,
                    Cursor::TabulationControl(_) => SEQ_ZERO,
                    Cursor::CharacterAbsolute(n) => surface.add_change(Change::CursorPosition {
                        x: Position::Absolute(n.as_zero_based() as usize),
                        y: Position::Relative(0),
                    }),
                    Cursor::CharacterAndLinePosition { line, col } => {
                        surface.add_change(Change::CursorPosition {
                            x: Position::Absolute(col.as_zero_based() as usize),
                            y: Position::Absolute(line.as_zero_based() as usize),
                        })
                    }
                    Cursor::CharacterPositionForward(n) => {
                        surface.add_change(Change::CursorPosition {
                            x: Position::Relative(n as isize),
                            y: Position::Relative(0),
                        })
                    }
                    Cursor::CharacterPositionBackward(n) => {
                        surface.add_change(Change::CursorPosition {
                            x: Position::Relative(-(n as isize)),
                            y: Position::Relative(0),
                        })
                    }
                    Cursor::CharacterPositionAbsolute(n) => {
                        surface.add_change(Change::CursorPosition {
                            x: Position::Absolute(n.as_zero_based() as usize),
                            y: Position::Relative(0),
                        })
                    }
                    Cursor::LinePositionForward(n) => surface.add_change(Change::CursorPosition {
                        x: Position::Relative(0),
                        y: Position::Relative(n as isize),
                    }),
                    Cursor::LinePositionBackward(n) => surface.add_change(Change::CursorPosition {
                        x: Position::Relative(0),
                        y: Position::Relative(-(n as isize)),
                    }),
                    Cursor::LinePositionAbsolute(n) => surface.add_change(Change::CursorPosition {
                        x: Position::Absolute(0),
                        y: Position::Absolute(n as usize),
                    }),
                    Cursor::Up(n) => surface.add_change(Change::CursorPosition {
                        x: Position::Relative(0),
                        y: Position::Relative(-(n as isize)),
                    }),
                    Cursor::Down(n) => surface.add_change(Change::CursorPosition {
                        x: Position::Relative(0),
                        y: Position::Relative(n as isize),
                    }),
                    Cursor::Right(n) => surface.add_change(Change::CursorPosition {
                        x: Position::Relative(n as isize),
                        y: Position::Relative(0),
                    }),
                    Cursor::Left(n) => surface.add_change(Change::CursorPosition {
                        x: Position::Relative(-(n as isize)),
                        y: Position::Relative(0),
                    }),
                    Cursor::NextLine(n) => surface.add_change(Change::CursorPosition {
                        x: Position::Absolute(0),
                        y: Position::Relative(n as isize),
                    }),
                    Cursor::PrecedingLine(n) => surface.add_change(Change::CursorPosition {
                        x: Position::Absolute(0),
                        y: Position::Relative(-(n as isize)),
                    }),
                    Cursor::Position { line, col } => surface.add_change(Change::CursorPosition {
                        x: Position::Absolute(col.as_zero_based() as usize),
                        y: Position::Absolute(line.as_zero_based() as usize),
                    }),
                    Cursor::SaveCursor => {
                        st.positions.push(surface.cursor_position());
                        SEQ_ZERO
                    }
                    Cursor::RestoreCursor => {
                        if let Some((x, y)) = st.positions.pop() {
                            surface.add_change(Change::CursorPosition {
                                x: Position::Absolute(x),
                                y: Position::Absolute(y),
                            })
                        } else {
                            SEQ_ZERO
                        }
                    }
                    Cursor::LineTabulation(_) => SEQ_ZERO,
                    Cursor::SetTopAndBottomMargins { .. } => SEQ_ZERO,
                    Cursor::SetLeftAndRightMargins { .. } => SEQ_ZERO,
                    Cursor::CursorStyle(_) => SEQ_ZERO,
                    Cursor::ActivePositionReport { .. } => SEQ_ZERO,
                    Cursor::RequestActivePositionReport => SEQ_ZERO,
                },
                _ => SEQ_ZERO,
            },
            _ => SEQ_ZERO,
        }
    }
}

#[derive(Debug, Default)]
struct State {
    positions: Vec<(usize, usize)>,
}

fn with_timeout<'scope, 'env, R, F>(
    timeout: Option<Duration>,
    mut killer: Box<dyn ChildKiller + Send + Sync>,
    s: &'scope thread::Scope<'scope, 'env>,
    f: F,
) -> R
where
    F: FnOnce() -> R,
{
    if let Some(timeout) = timeout {
        let t = s.spawn(move || {
            thread::sleep(timeout);
            let _ = killer.kill();
        });
        let result = f();
        t.thread().unpark();
        t.join().unwrap();
        result
    } else {
        f()
    }
}

fn tabulate(pos: usize, n: usize) -> usize {
    pos + (TAB_STOP * n - pos % TAB_STOP)
}

fn tabulate_back(pos: usize, n: usize) -> usize {
    pos.saturating_sub(pos % TAB_STOP + TAB_STOP * (n - 1))
}

const TAB_STOP: usize = 8;
