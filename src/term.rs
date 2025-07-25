use std::{
    collections::HashMap,
    io::{self, BufRead, BufReader, BufWriter},
    mem,
    sync::{
        Arc, Mutex,
        mpsc::{Sender, channel},
    },
    thread,
    time::Duration,
};

use anyhow::{Context, Result};
use num_traits::FromPrimitive;
use portable_pty::{ChildKiller, CommandBuilder, PtySize, native_pty_system};
use termwiz::{
    cell::AttributeChange,
    color::SrgbaTuple,
    escape::{
        Action, CSI, ControlCode, OneBased, OperatingSystemCommand,
        csi::{Cursor, Sgr},
        osc::{ColorOrQuery, DynamicColorNumber},
        parser::Parser,
    },
    surface::{Change, Position, SEQ_ZERO, SequenceNo, Surface},
};

/// Options for configuring the terminal.
#[derive(Debug, Default)]
pub struct Options {
    pub cols: Option<u16>,
    pub rows: Option<u16>,
    pub background: Option<SrgbaTuple>,
    pub foreground: Option<SrgbaTuple>,
    pub env: HashMap<String, String>,
}

/// Represents a terminal with a surface, parser, state, and size.
pub struct Terminal {
    env: HashMap<String, String>,
    surface: Surface,
    parser: Parser,
    state: State,
    size: PtySize,
}

impl Terminal {
    /// Creates a new terminal with the given options.
    pub fn new(options: Options) -> Self {
        let cols = options.cols.unwrap_or(80);
        let rows = options.rows.unwrap_or(24);
        let background = options
            .background
            .unwrap_or(SrgbaTuple::from_hsla(0.0, 0.0, 0.9, 1.0));
        let foreground = options
            .foreground
            .unwrap_or(SrgbaTuple::from_hsla(0.0, 0.0, 0.75, 1.0));

        // Define terminal size.
        let size = PtySize {
            cols,
            rows,
            pixel_width: 0,
            pixel_height: 0,
        };

        Self {
            env: options.env,
            surface: Surface::new(cols.into(), rows.into()),
            parser: Parser::new(),
            state: State::new(background, foreground),
            size,
        }
    }

    /// Returns a reference to the terminal's surface.
    pub fn surface(&self) -> &Surface {
        &self.surface
    }

    /// Returns the background color of the terminal.
    pub fn background(&self) -> SrgbaTuple {
        self.state.background
    }

    /// Returns the foreground color of the terminal.
    pub fn foreground(&self) -> SrgbaTuple {
        self.state.foreground
    }

    /// Feeds input from the reader to the terminal and writes output to the writer.
    pub fn feed(&mut self, mut reader: impl BufRead, mut writer: impl io::Write) -> Result<()> {
        loop {
            let buffer = reader.fill_buf().context("error reading PTY")?;
            if buffer.is_empty() {
                return Ok(());
            }

            self.parser.parse(buffer, |action| {
                let seq =
                    Self::apply_action(&mut self.surface, &mut self.state, &mut writer, action);
                self.surface.flush_changes_older_than(seq);
            });

            let len = buffer.len();
            reader.consume(len);
        }
    }

    /// Runs a command in the terminal with an optional timeout.
    pub fn run(&mut self, mut cmd: CommandBuilder, timeout: Option<Duration>) -> Result<()> {
        for (key, value) in &self.env {
            cmd.env(key, value);
        }

        if cmd.get_cwd().is_none() {
            cmd.cwd(".");
        }

        // Create a PTY pair using portable-pty.
        let pty = native_pty_system();
        let pair = pty.openpty(self.size)?;

        let reader = BufReader::new(pair.master.try_clone_reader()?);
        let mut child = pair.slave.spawn_command(cmd)?;
        let killer = child.clone_killer();

        let writer = pair.master.take_writer()?;
        let writer = ThreadedWriter::new(Box::new(writer));
        let writer = DetachableWriter::new(Box::new(BufWriter::new(writer)));

        thread::scope(|s| {
            let wr = writer.clone();
            let thread = s.spawn(move || self.feed(reader, wr));

            with_timeout(timeout, killer, s, || child.wait())?;

            log::debug!("drop writer");
            writer.detach().flush()?;

            log::debug!("drop child");
            drop(child);

            log::debug!("drop pty pair");
            drop(pair);

            log::debug!("join processing thread");
            thread.join().unwrap()
        })?;

        Ok(())
    }

    /// Applies an action to the terminal's surface and state, and writes output to the writer.
    fn apply_action(
        surface: &mut Surface,
        st: &mut State,
        mut writer: impl io::Write,
        action: Action,
    ) -> SequenceNo {
        match action {
            Action::Print(ch) => surface.add_change(ch),
            Action::PrintString(s) => surface.add_change(s),
            Action::Control(code) => match code {
                ControlCode::LineFeed | ControlCode::VerticalTab | ControlCode::FormFeed => {
                    surface.add_change("\r\n")
                }
                ControlCode::CarriageReturn => surface.add_change("\r"),
                ControlCode::HorizontalTab => surface.add_change(Change::CursorPosition {
                    x: Position::Absolute(tabulate(surface.cursor_position().0, 1)),
                    y: Position::Relative(0),
                }),
                ControlCode::Backspace => {
                    surface.add_change(Change::CursorPosition {
                        x: Position::Relative(-1),
                        y: Position::Relative(0),
                    });
                    surface.add_change(" ");
                    surface.add_change(Change::CursorPosition {
                        x: Position::Relative(-1),
                        y: Position::Relative(0),
                    })
                }
                _ => {
                    log::debug!("unsupported: Control({code:?})");
                    SEQ_ZERO
                }
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
                    Cursor::RequestActivePositionReport => {
                        log::debug!("RequestActivePositionReport");
                        let col = OneBased::from_zero_based(surface.cursor_position().0 as u32);
                        let line = OneBased::from_zero_based(surface.cursor_position().1 as u32);
                        let report = CSI::Cursor(Cursor::ActivePositionReport { line, col });
                        log::debug!("ActivePositionReport {report:?}");
                        write!(writer, "{report}").ok();
                        writer.flush().ok();
                        SEQ_ZERO
                    }
                },
                CSI::Device(device) => {
                    log::debug!("unsupported: CSI::Device({device:?})");
                    SEQ_ZERO
                }
                CSI::Mode(mode) => {
                    log::debug!("unsupported: CSI::Mode({mode:?})");
                    SEQ_ZERO
                }
                CSI::Edit(edit) => {
                    log::debug!("unsupported: CSI::Edit({edit:?})");
                    SEQ_ZERO
                }
                CSI::Window(window) => {
                    log::debug!("unsupported: CSI::Window({window:?})");
                    SEQ_ZERO
                }
                CSI::Mouse(mouse) => {
                    log::debug!("unsupported: CSI::Mouse({mouse:?})");
                    SEQ_ZERO
                }
                CSI::Keyboard(keyboard) => {
                    log::debug!("unsupported: CSI::Keyboard({keyboard:?})");
                    SEQ_ZERO
                }
                CSI::SelectCharacterPath(p, n) => {
                    log::debug!("unsupported: CSI::SelectCharacterPath({p:?}, {n:?})");
                    SEQ_ZERO
                }
                CSI::Unspecified(v) => {
                    log::debug!("unsupported: CSI::Unspecified({v:?})");
                    SEQ_ZERO
                }
            },
            Action::DeviceControl(mode) => {
                log::debug!("unsupported: DeviceControl({mode:?})");
                SEQ_ZERO
            }
            Action::OperatingSystemCommand(cmd) => match *cmd {
                OperatingSystemCommand::ChangeDynamicColors(first_color, colors) => {
                    let mut idx: u8 = first_color as u8;
                    for color in colors {
                        let which_color: Option<DynamicColorNumber> = FromPrimitive::from_u8(idx);
                        log::debug!("ChangeDynamicColors({which_color:?}): {color:?}");
                        if let Some(which_color) = which_color {
                            let mut set_or_query = |target: &mut SrgbaTuple| match color {
                                ColorOrQuery::Query => {
                                    let response = OperatingSystemCommand::ChangeDynamicColors(
                                        which_color,
                                        vec![ColorOrQuery::Color(*target)],
                                    );
                                    log::debug!("Color Query response {response:?}");
                                    write!(writer, "{response}").ok();
                                    writer.flush().ok();
                                }
                                ColorOrQuery::Color(c) => {
                                    log::debug!("{which_color:?} set to {c}", c = c.to_string());
                                    *target = c
                                }
                            };
                            match which_color {
                                DynamicColorNumber::TextForegroundColor => {
                                    set_or_query(&mut st.foreground)
                                }
                                DynamicColorNumber::TextBackgroundColor => {
                                    set_or_query(&mut st.background)
                                }
                                DynamicColorNumber::TextCursorColor => unimplemented!(),
                                DynamicColorNumber::HighlightForegroundColor => unimplemented!(),
                                DynamicColorNumber::HighlightBackgroundColor => unimplemented!(),
                                DynamicColorNumber::MouseForegroundColor
                                | DynamicColorNumber::MouseBackgroundColor
                                | DynamicColorNumber::TektronixForegroundColor
                                | DynamicColorNumber::TektronixBackgroundColor
                                | DynamicColorNumber::TektronixCursorColor => unimplemented!(),
                            }
                        }
                        idx += 1;
                    }
                    SEQ_ZERO
                }
                _ => {
                    log::debug!("unsupported: OperatingSystemCommand({cmd:?})");
                    SEQ_ZERO
                }
            },
            Action::Esc(esc) => match esc {
                termwiz::escape::Esc::Code(termwiz::escape::EscCode::StringTerminator) => SEQ_ZERO,
                _ => {
                    log::debug!("unsupported: Esc({esc:?})");
                    SEQ_ZERO
                }
            },
            Action::XtGetTcap(cap) => {
                log::debug!("unsupported: XtGetTcap({cap:?})");
                SEQ_ZERO
            }
            Action::Sixel(sixel) => {
                log::debug!("unsupported: Sixel({sixel:?})");
                SEQ_ZERO
            }
            Action::KittyImage(image) => {
                log::debug!("unsupported: KittyImage({image:?})");
                SEQ_ZERO
            }
        }
    }
}

/// Represents the state of the terminal, including cursor positions and colors.
#[derive(Debug)]
struct State {
    positions: Vec<(usize, usize)>,
    background: SrgbaTuple,
    foreground: SrgbaTuple,
}

impl State {
    /// Creates a new state with the given background and foreground colors.
    fn new(background: SrgbaTuple, foreground: SrgbaTuple) -> Self {
        Self {
            background,
            foreground,
            positions: Vec::new(),
        }
    }
}

/// A writer that sends data to a separate thread for writing.
struct ThreadedWriter {
    sender: Sender<WriterMessage>,
}

impl ThreadedWriter {
    /// Creates a new threaded writer.
    fn new(mut writer: Box<dyn io::Write + Send>) -> Self {
        let (sender, receiver) = channel::<WriterMessage>();

        std::thread::spawn(move || {
            while let Ok(msg) = receiver.recv() {
                match msg {
                    WriterMessage::Data(buf) => {
                        if writer.write(&buf).is_err() {
                            break;
                        }
                    }
                    WriterMessage::Flush => {
                        if writer.flush().is_err() {
                            break;
                        }
                    }
                }
            }
        });

        Self { sender }
    }
}

impl io::Write for ThreadedWriter {
    fn write(&mut self, buf: &[u8]) -> io::Result<usize> {
        self.sender
            .send(WriterMessage::Data(buf.to_vec()))
            .map_err(|err| io::Error::new(io::ErrorKind::BrokenPipe, err))?;
        Ok(buf.len())
    }

    fn flush(&mut self) -> io::Result<()> {
        self.sender
            .send(WriterMessage::Flush)
            .map_err(|err| io::Error::new(io::ErrorKind::BrokenPipe, err))?;
        Ok(())
    }
}

/// Messages that can be sent to the threaded writer.
enum WriterMessage {
    Data(Vec<u8>),
    Flush,
}

/// A writer that can be detached and replaced.
#[derive(Clone)]
struct DetachableWriter {
    inner: Arc<Mutex<Box<dyn io::Write + Send>>>,
}

impl DetachableWriter {
    /// Creates a new detachable writer.
    fn new(writer: Box<dyn io::Write + Send>) -> Self {
        Self {
            inner: Arc::new(Mutex::new(writer)),
        }
    }

    /// Detaches the current writer and replaces it with a sink.
    fn detach(&self) -> Box<dyn io::Write + Send> {
        self.replace(Box::new(io::sink()))
    }

    /// Replaces the current writer with a new one.
    fn replace(&self, writer: Box<dyn io::Write + Send>) -> Box<dyn io::Write + Send> {
        let mut inner = self.inner.lock().unwrap();
        mem::replace(&mut inner, writer)
    }
}

impl io::Write for DetachableWriter {
    fn write(&mut self, buf: &[u8]) -> io::Result<usize> {
        self.inner.lock().unwrap().write(buf)
    }

    fn flush(&mut self) -> io::Result<()> {
        self.inner.lock().unwrap().flush()
    }
}

fn with_timeout<'scope, R, F>(
    timeout: Option<Duration>,
    mut killer: Box<dyn ChildKiller + Send + Sync>,
    s: &'scope thread::Scope<'scope, '_>,
    f: F,
) -> R
where
    F: FnOnce() -> R,
{
    if let Some(timeout) = timeout {
        let t = s.spawn(move || {
            thread::park_timeout(timeout);
            let _ = killer.kill();
        });
        let result = f();
        log::debug!("unpark timeout thread");
        t.thread().unpark();
        log::debug!("join timeout thread");
        t.join().unwrap();
        log::debug!("done");
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
