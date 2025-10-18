use std::{
    collections::{HashMap, VecDeque},
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
    surface::{Change, Line, Position, SEQ_ZERO, SequenceNo, Surface, change::ChangeSequence},
};
use unicode_width::{UnicodeWidthChar, UnicodeWidthStr};

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
            state: State::new(background, foreground, rows as usize),
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
                let (bx, by) = self.surface.cursor_position();
                log::debug!("parse: action={:?}, cursor_before=({}, {})", action, bx, by);
                let seq = Self::apply_action_with_autowrap(
                    &mut self.surface,
                    &mut self.state,
                    &mut writer,
                    action,
                );
                let (ax, ay) = self.surface.cursor_position();
                log::debug!("parse: cursor_after=({}, {}), seq={}", ax, ay, seq);
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

    pub fn recommended_width(&self) -> u16 {
        self.process_logical_lines_with_accumulator(0, |max_width, width| {
            if width > *max_width {
                *max_width = width;
            }
        }) as u16
    }

    /// Process logical lines from the transcript with a flexible accumulator pattern.
    /// Iterates over transcript references without cloning for optimal performance.
    fn process_logical_lines_with_accumulator<T, F>(&self, mut accumulator: T, mut callback: F) -> T
    where
        F: FnMut(&mut T, usize),
    {
        let mut logical_width = 0usize;
        let mut prev_wrapped = false;

        // Process scrollback lines (already owned, borrow as &Line)
        for line in &self.state.scrollback {
            let this_wrapped = line.last_cell_was_wrapped();

            if prev_wrapped {
                // Previous row wrapped, so this continues the same logical line
                logical_width += Self::trimmed_line_width(line);
            } else {
                // Previous row did not wrap; finish that logical line and start a new one
                if logical_width > 0 {
                    callback(&mut accumulator, logical_width);
                }
                logical_width = Self::trimmed_line_width(line);
            }

            prev_wrapped = this_wrapped;
        }

        // Process visible lines (as Cow references, avoid into_owned())
        for cow_line in self.surface.screen_lines() {
            let line = cow_line.as_ref();
            let this_wrapped = line.last_cell_was_wrapped();

            if prev_wrapped {
                // Previous row wrapped, so this continues the same logical line
                logical_width += Self::trimmed_line_width(line);
            } else {
                // Previous row did not wrap; finish that logical line and start a new one
                if logical_width > 0 {
                    callback(&mut accumulator, logical_width);
                }
                logical_width = Self::trimmed_line_width(line);
            }

            prev_wrapped = this_wrapped;
        }

        // Don't forget the final logical line
        if logical_width > 0 {
            callback(&mut accumulator, logical_width);
        }

        accumulator
    }

    pub fn set_width(&mut self, width: u16) {
        // Rewrap using keep-height strategy; do not change the viewport height here.
        self.rewrap_surface(width as usize);
        // Update only reported columns
        self.size.cols = width;
    }

    pub fn recommended_height(&self) -> u16 {
        let (width, _) = self.surface.dimensions();
        let mut total_rows = 0;
        let mut last_logical_empty = false;
        let mut trailing_empty_rows = 0;

        self.process_logical_lines_with_accumulator((), |_acc, logical_width| {
            // Calculate rows needed for this logical line using ceiling division
            let rows_needed = if logical_width == 0 {
                1 // Empty logical lines still take one row
            } else {
                logical_width.div_ceil(width)
            };

            // Track if this logical line is empty (for trailing trimming)
            let is_empty = logical_width == 0;
            if is_empty {
                trailing_empty_rows += rows_needed;
            } else {
                // Non-empty line found, reset trailing counter
                total_rows += trailing_empty_rows + rows_needed;
                trailing_empty_rows = 0;
            }
            last_logical_empty = is_empty;
        });

        // Don't count trailing empty logical lines
        total_rows as u16
    }

    pub fn set_height(&mut self, height: u16) {
        let w = self.surface.dimensions().0;
        self.unscroll_to_window(w, height as usize);
        self.size.rows = height;
    }

    /// Build owned transcript lines (scrollback + visible).
    /// This clones data and is used when owned Lines are needed for operations
    /// like wrapping. For read-only operations, consider transcript_line_refs().
    fn transcript_lines(&self) -> Vec<Line> {
        let mut out: Vec<Line> = Vec::new();
        // append scrollback lines (chronological order)
        for ln in &self.state.scrollback {
            out.push(ln.clone());
        }
        // append current visible rows
        for cow in self.surface.screen_lines() {
            out.push(cow.into_owned());
        }
        out
    }

    fn trimmed_line_width(line: &Line) -> usize {
        let mut width = 0usize;
        for cell in line.visible_cells() {
            if cell.str().trim().is_empty() {
                continue;
            }
            let end = cell.cell_index() + cell.width().max(1);
            if end > width {
                width = end;
            }
        }
        width
    }

    fn join_logical_lines(&self, lines: Vec<Line>) -> Vec<Line> {
        self.join_logical_lines_from_iter(lines.into_iter())
    }

    /// Join logical lines from an iterator of owned Lines.
    /// This is the core logical line joining implementation that both
    /// join_logical_lines() and other methods can reuse.
    fn join_logical_lines_from_iter<I>(&self, lines: I) -> Vec<Line>
    where
        I: Iterator<Item = Line>,
    {
        let seq = self.surface.current_seqno();
        let mut out: Vec<Line> = Vec::new();
        let mut current: Option<Line> = None;
        let mut prev_wrapped = false;

        for ln in lines {
            let this_wrapped = ln.last_cell_was_wrapped();

            if let Some(acc) = current.as_mut() {
                if prev_wrapped {
                    // Previous physical row wrapped, so this row continues the same logical line
                    acc.append_line(ln, seq);
                } else {
                    // Previous physical row did not wrap; finish that logical line and start a new one
                    out.push(current.take().unwrap());
                    current = Some(ln);
                }
            } else {
                current = Some(ln);
            }

            // Track whether the current physical row wrapped, to decide how to treat the next row
            prev_wrapped = this_wrapped;
        }

        if let Some(acc) = current.take() {
            out.push(acc);
        }
        out
    }

    /// Unscroll the transcript to a given window:
    /// - Reflow the full transcript to `new_width`
    /// - Materialize the bottom `window_height` rows on the Surface
    /// - Rebuild scrollback and refresh wrap flags for visible rows
    fn unscroll_to_window(&mut self, new_width: usize, window_height: usize) {
        let seq = self.surface.current_seqno();

        // 1) Build logical lines from full transcript (scrollback + visible).
        let logicals = self.join_logical_lines(self.transcript_lines());

        // 2) Re-wrap each logical line to the new width.
        let mut reflowed: Vec<Line> = Vec::new();
        for ln in logicals {
            reflowed.extend(ln.wrap(new_width, seq));
        }

        // 3) Trim trailing blank rows to avoid empty tail.
        while reflowed
            .last()
            .map(|ln| ln.visible_cells().all(|c| c.str().trim().is_empty()))
            .unwrap_or(false)
        {
            reflowed.pop();
        }

        // 4) Compute bottom window slice for the requested height.
        let total = reflowed.len();
        let start = total.saturating_sub(window_height);

        // 5) Rebuild scrollback from the portion above the visible window.
        self.state.scrollback.clear();
        for ln in reflowed.iter().take(start) {
            self.state.push_scrollback_line(ln.clone());
        }

        // 6) Resize surface to the requested width and height.
        self.surface.resize(new_width, window_height);

        // 7) Render the bottom window rows into the surface.
        for row in 0..window_height {
            if let Some(ln) = reflowed.get(start + row) {
                self.replace_row_with_line(row, ln);
            }
        }

        // 8) Update wrap flags for visible rows.
        self.state.ensure_height(window_height);
        for row in 0..window_height {
            if let Some(flag) = self.state.wrap_flags.get_mut(row) {
                let wrapped = reflowed
                    .get(start + row)
                    .map(|ln| ln.last_cell_was_wrapped())
                    .unwrap_or(false);
                *flag = wrapped;
            }
        }
    }

    /// Rewrap the whole surface to `new_width`, merging previously wrapped rows,
    /// while keeping the current viewport height unchanged.
    fn rewrap_surface(&mut self, new_width: usize) -> usize {
        let (_, h) = self.surface.dimensions();
        self.unscroll_to_window(new_width, h);
        self.surface.current_seqno()
    }

    fn replace_row_with_line(&mut self, row: usize, ln: &Line) {
        let (w, _) = self.surface.dimensions();
        // Preserve current cursor position to avoid interfering with ongoing printing
        let (cur_x, cur_y) = self.surface.cursor_position();

        // Create a 1-row temp screen that contains exactly the desired line content.
        let mut tmp = Surface::new(w, 1);

        // Emit a compact stream of changes for ln into tmp.
        let mut seq = ChangeSequence::new(1, w);
        let mut last_attr = None;

        for cell in ln.visible_cells() {
            let x = cell.cell_index();

            // Move cursor to the correct x for this run
            seq.add(Change::CursorPosition {
                x: Position::Absolute(x),
                y: Position::Absolute(0),
            });

            // Update attributes only when they change
            if last_attr.as_ref() != Some(cell.attrs()) {
                seq.add(Change::AllAttributes(cell.attrs().clone()));
                last_attr = Some(cell.attrs().clone());
            }

            // Append the grapheme(s)
            seq.add(Change::Text(cell.str().to_owned()));
        }

        tmp.add_changes(seq.consume());

        // Compute minimal diff for that single row and apply it to the real surface
        let changes = self.surface.diff_region(0, row, w, 1, &tmp, 0, 0);
        self.surface.add_changes(changes);

        // Restore cursor position
        self.surface.add_change(Change::CursorPosition {
            x: Position::Absolute(cur_x),
            y: Position::Absolute(cur_y),
        });
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
                    log::debug!("Control: LF/VT/FF -> CRLF");
                    surface.add_change("\r\n")
                }
                ControlCode::CarriageReturn => {
                    log::debug!("Control: CR");
                    surface.add_change("\r")
                }
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
    wrap_flags: Vec<bool>,
    scrollback: VecDeque<Line>,
    scrollback_limit: usize,
}

impl State {
    /// Creates a new state with the given background and foreground colors.
    fn new(background: SrgbaTuple, foreground: SrgbaTuple, height: usize) -> Self {
        Self {
            background,
            foreground,
            positions: Vec::new(),
            wrap_flags: vec![false; height],
            scrollback: VecDeque::new(),
            scrollback_limit: 10_000,
        }
    }

    /// Ensure the wrap ledger has the specified height, clearing new slots.
    fn ensure_height(&mut self, height: usize) {
        if self.wrap_flags.len() != height {
            self.wrap_flags.resize(height, false);
        }
    }

    /// Rotate the ledger upward by one row to mirror a screen scroll up.
    /// The new bottom row is cleared (no wrap).
    fn rotate_on_scroll(&mut self) {
        if !self.wrap_flags.is_empty() {
            self.wrap_flags.rotate_left(1);
            if let Some(last) = self.wrap_flags.last_mut() {
                *last = false;
            }
        }
    }

    /// Push a line into scrollback and enforce the limit.
    fn push_scrollback_line(&mut self, line: Line) {
        self.scrollback.push_back(line);
        self.trim_scrollback_to_limit();
    }

    /// Ensure scrollback does not exceed the configured limit.
    fn trim_scrollback_to_limit(&mut self) {
        while self.scrollback.len() > self.scrollback_limit {
            self.scrollback.pop_front();
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

impl Terminal {
    /// Apply an action while detecting automatic wrapping at the right margin.
    /// If printing caused the cursor to advance to later rows (without an explicit LF),
    /// mark those crossed rows as soft-wrapped by setting the last-cell wrapped bit.
    fn apply_action_with_autowrap(
        surface: &mut Surface,
        st: &mut State,
        mut writer: impl io::Write,
        action: Action,
    ) -> SequenceNo {
        // Cursor prior to applying the action
        let (x0, y0) = surface.cursor_position();
        log::debug!(
            "autowrap: before action={:?}, cursor=({}, {})",
            action,
            x0,
            y0
        );

        // Capture scrolled-out rows (scrollback) before applying actions that will scroll
        {
            let (w, h) = surface.dimensions();
            match &action {
                Action::Print(ch) => {
                    if *ch != '\n'
                        && *ch != '\r'
                        && let Some(ch_width) = UnicodeWidthChar::width(*ch)
                        && ch_width > 0
                    {
                        let cap = w.saturating_sub(x0);
                        if y0 == h.saturating_sub(1) && ch_width > cap {
                            // one row will scroll out from the top
                            if let Some(cow) = surface.screen_lines().first() {
                                st.push_scrollback_line(cow.clone().into_owned());
                            }
                        }
                    }
                }
                Action::PrintString(s) => {
                    let disp_width = UnicodeWidthStr::width(s.as_str());
                    if disp_width > 0 {
                        let cap = w.saturating_sub(x0);
                        let total_wraps = if disp_width > cap {
                            1 + (disp_width - cap - 1) / w
                        } else {
                            0
                        };
                        if total_wraps > 0 {
                            let rows_available = h.saturating_sub(1).saturating_sub(y0);
                            let wraps_before_bottom = total_wraps.min(rows_available);
                            let bottom_wraps = total_wraps.saturating_sub(wraps_before_bottom);
                            // Push the top N rows that will be scrolled out
                            for i in 0..bottom_wraps {
                                if let Some(cow) = surface.screen_lines().get(i) {
                                    st.push_scrollback_line(cow.clone().into_owned());
                                }
                            }
                        }
                    }
                }
                Action::Control(code) => {
                    if matches!(
                        code,
                        ControlCode::LineFeed | ControlCode::VerticalTab | ControlCode::FormFeed
                    ) && y0 == h.saturating_sub(1)
                        && let Some(cow) = surface.screen_lines().first()
                    {
                        st.push_scrollback_line(cow.clone().into_owned());
                    }
                }
                _ => {}
            }
        }

        // Apply the original action
        let seq = Self::apply_action(surface, st, &mut writer, action.clone());

        let (x1, y1) = surface.cursor_position();
        log::debug!(
            "autowrap: after action={:?}, cursor=({}, {}), seq={}",
            action,
            x1,
            y1,
            seq
        );

        // Keep the ledger height in sync with the surface.
        let (w, h) = surface.dimensions();
        st.ensure_height(h);

        // If this was printing and we crossed rows, that indicates autowraps.
        // Additionally, detect the bottom-scroll case where wrapping occurs but y doesn't change:
        // when xpos exceeded width on entry (x0 >= width) and after printing we observed x1 < x0.
        match action {
            Action::Print(ch) => {
                // Width-aware single-char wrap detection to mark/rotate the wrap ledger precisely.
                if ch != '\n' && ch != '\r' {
                    let ch_width = UnicodeWidthChar::width(ch).unwrap_or(0);
                    if ch_width > 0 {
                        let cap = w.saturating_sub(x0);
                        if ch_width > cap {
                            // This print will trigger a wrap
                            if y0 < h.saturating_sub(1) {
                                // Wrapped within buffer: mark current row y0
                                if let Some(flag) = st.wrap_flags.get_mut(y0) {
                                    *flag = true;
                                }
                                Self::mark_row_soft_wrapped(surface, y0, seq);
                            } else {
                                // Bottom scroll wrap: rotate ledger and mark h-2
                                st.rotate_on_scroll();
                                if h >= 2 {
                                    let r = h - 2;
                                    if let Some(flag) = st.wrap_flags.get_mut(r) {
                                        *flag = true;
                                    }
                                    Self::mark_row_soft_wrapped(surface, r, seq);
                                }
                            }
                        }
                    }
                }
            }
            Action::PrintString(ref s) => {
                // Estimate wraps for this chunk using display width.
                let disp_width = UnicodeWidthStr::width(s.as_str());
                let cap = w.saturating_sub(x0);
                if disp_width > 0 {
                    let total_wraps = if disp_width > cap {
                        1 + (disp_width - cap - 1) / w
                    } else {
                        0
                    };
                    if total_wraps > 0 {
                        let rows_available = h.saturating_sub(1).saturating_sub(y0);
                        let wraps_before_bottom = total_wraps.min(rows_available);
                        let bottom_wraps = total_wraps.saturating_sub(wraps_before_bottom);
                        // First, mark wraps within the buffer before hitting bottom.
                        for r in y0..(y0 + wraps_before_bottom) {
                            if let Some(flag) = st.wrap_flags.get_mut(r) {
                                *flag = true;
                            }
                            Self::mark_row_soft_wrapped(surface, r, seq);
                        }
                        // Then, handle bottom scroll wraps by rotating the ledger and marking h-2.
                        for _ in 0..bottom_wraps {
                            st.rotate_on_scroll();
                            if h >= 2 {
                                let r = h - 2;
                                if let Some(flag) = st.wrap_flags.get_mut(r) {
                                    *flag = true;
                                }
                                Self::mark_row_soft_wrapped(surface, r, seq);
                            }
                        }
                    }
                }
            }
            Action::Control(code) => {
                // For LF/VT/FF at bottom: a scroll up occurs and y may remain unchanged.
                if matches!(
                    code,
                    ControlCode::LineFeed | ControlCode::VerticalTab | ControlCode::FormFeed
                ) && y0 == h.saturating_sub(1)
                {
                    log::debug!(
                        "autowrap: detected scroll caused by {:?}; rotating ledger",
                        code
                    );
                    st.rotate_on_scroll();
                }
            }
            _ => {}
        }

        seq
    }

    /// Mark a specific row as soft-wrapped by setting the wrapped bit on its last cell.
    /// Writes the updated line back to the surface via a minimal diff.
    fn mark_row_soft_wrapped(surface: &mut Surface, row: usize, _seq: SequenceNo) {
        let (w, h) = surface.dimensions();
        if row >= h || w == 0 {
            log::debug!("mark_row_soft_wrapped: row {} out of range (h={})", row, h);
            return;
        }

        // In-place: mark the wrapped flag on the last visible cell in the row.
        // First compute the last visible cell index without holding a mutable borrow.
        let idx = surface
            .screen_lines()
            .get(row)
            .and_then(|line| line.visible_cells().last().map(|c| c.cell_index()))
            .unwrap_or(w.saturating_sub(1));

        // Now take a mutable borrow and flip the bit in place.
        let mut rows = surface.screen_cells();
        let cells = &mut rows[row];

        let was = cells[idx].attrs().wrapped();
        cells[idx].attrs_mut().set_wrapped(true);
        log::debug!(
            "mark_row_soft_wrapped: row {} cell {} wrapped: {} -> true",
            row,
            idx,
            was
        );
    }
}

#[cfg(test)]
mod tests {
    // Unified tests live at file end. Please don't add another `mod tests` elsewhere.
    use super::*;
    use std::collections::HashMap;
    use std::io::Cursor;

    #[test]
    fn test_autowrap_marks_wrapped_lines() {
        let mut term = Terminal::new(Options {
            cols: Some(3),
            rows: Some(3),
            background: None,
            foreground: None,
            env: HashMap::new(),
        });

        let mut reader = Cursor::new(b"abcdef".as_ref());
        let mut writer = Vec::new();
        term.feed(&mut reader, &mut writer).unwrap();

        let rw = term.recommended_width();
        assert_eq!(rw, 6, "recommended width should be 6 to fit");

        let lines = term.surface().screen_lines();
        assert!(
            lines[0].last_cell_was_wrapped(),
            "first line should be marked as soft-wrapped"
        );

        let l0: String = lines[0]
            .visible_cells()
            .map(|c| c.str().to_string())
            .collect();
        let l1: String = lines[1]
            .visible_cells()
            .map(|c| c.str().to_string())
            .collect();

        assert_eq!(l0.trim_end(), "abc");
        assert_eq!(l1.trim_end(), "def");
    }

    #[test]
    fn test_explicit_newline_not_marked_wrapped() {
        let mut term = Terminal::new(Options {
            cols: Some(5),
            rows: Some(3),
            background: None,
            foreground: None,
            env: HashMap::new(),
        });

        let mut reader = Cursor::new(b"abc\ndef".as_ref());
        let mut writer = Vec::new();
        term.feed(&mut reader, &mut writer).unwrap();

        let lines = term.surface().screen_lines();
        assert!(
            !lines[0].last_cell_was_wrapped(),
            "explicit newline must not be marked as soft-wrapped"
        );
    }

    #[test]
    fn test_autowrap_marks_on_bottom_scroll() {
        // width=3, height=2 to force bottom scroll on the 7th char
        let mut term = Terminal::new(Options {
            cols: Some(3),
            rows: Some(2),
            background: None,
            foreground: None,
            env: HashMap::new(),
        });

        let mut reader = std::io::Cursor::new(b"abcdefg".as_ref());
        let mut writer = Vec::new();
        term.feed(&mut reader, &mut writer).unwrap();

        let lines = term.surface().screen_lines();

        // After bottom scroll caused by 'g', the line that just wrapped (previous bottom row)
        // has moved up by one row. It is now row 0 and must be marked as wrapped.
        assert!(
            lines[0].last_cell_was_wrapped(),
            "row 0 should be soft-wrapped after bottom scroll"
        );

        // Validate visible content is not corrupted.
        let r0: String = lines[0]
            .visible_cells()
            .map(|c| c.str().to_string())
            .collect();
        let r1: String = lines[1]
            .visible_cells()
            .map(|c| c.str().to_string())
            .collect();

        assert_eq!(
            r0.trim_end(),
            "def",
            "row 0 should contain the wrapped prior bottom line"
        );
        assert_eq!(
            r1.trim_end(),
            "g",
            "row 1 should start with 'g' after scroll"
        );
    }

    #[test]
    fn test_multiple_bottom_scrolls_preserve_wrap_and_content() {
        // width=3, height=2, long input to trigger multiple bottom scrolls
        let mut term = Terminal::new(Options {
            cols: Some(3),
            rows: Some(2),
            background: None,
            foreground: None,
            env: HashMap::new(),
        });

        // 12 chars: will cause several wraps and two bottom scrolls
        let mut reader = std::io::Cursor::new(b"abcdefghijkl".as_ref());
        let mut writer = Vec::new();
        term.feed(&mut reader, &mut writer).unwrap();

        let lines = term.surface().screen_lines();

        // After multiple scrolls, we expect:
        //   row 0 == "ghi" (wrapped), row 1 == "jkl"
        let r0: String = lines[0]
            .visible_cells()
            .map(|c| c.str().to_string())
            .collect();
        let r1: String = lines[1]
            .visible_cells()
            .map(|c| c.str().to_string())
            .collect();

        assert_eq!(
            r0.trim_end(),
            "ghi",
            "row 0 content must be intact after multiple scrolls"
        );
        assert_eq!(
            r1.trim_end(),
            "jkl",
            "row 1 content must be intact after multiple scrolls"
        );

        // The row that just wrapped prior to the last scroll should be marked as wrapped.
        assert!(
            lines[0].last_cell_was_wrapped(),
            "row 0 should be soft-wrapped after multiple bottom scrolls"
        );
    }
    #[test]
    fn test_recommended_width_autowrap() {
        let mut term = Terminal::new(Options {
            cols: Some(3),
            rows: Some(3),
            background: None,
            foreground: None,
            env: HashMap::new(),
        });

        let mut reader = Cursor::new(b"abcdef".as_ref());
        let mut writer = Vec::new();
        term.feed(&mut reader, &mut writer).unwrap();

        // A single logical line "abcdef" should yield recommended_width = 6
        assert_eq!(term.recommended_width(), 6);
    }

    #[test]
    fn test_recommended_width_with_scrollback_optimization() {
        // Test that the optimized recommended_width implementation works correctly
        // with both scrollback and visible content, including wrapped lines
        let mut term = Terminal::new(Options {
            cols: Some(6),
            rows: Some(3),
            background: None,
            foreground: None,
            env: HashMap::new(),
        });

        // First line: "hello!" (6 chars, fits in one row)
        let mut reader = Cursor::new(b"hello!\n".as_ref());
        let mut writer = Vec::new();
        term.feed(&mut reader, &mut writer).unwrap();

        // Second line: "verylongline" (12 chars, wraps to 2 rows: "verylo" + "ngline")
        let mut reader = Cursor::new(b"verylongline\n".as_ref());
        term.feed(&mut reader, &mut writer).unwrap();

        // Third line: "short" (5 chars, fits in one row)
        let mut reader = Cursor::new(b"short\n".as_ref());
        term.feed(&mut reader, &mut writer).unwrap();

        // At this point we should have scrolled since we have more than 3 rows of content
        // The recommended width should be 12 (from "verylongline")
        assert_eq!(term.recommended_width(), 12);

        // Add another very long line to test with more scrollback content
        let mut reader = Cursor::new(b"superlonglinethatis21".as_ref());
        term.feed(&mut reader, &mut writer).unwrap();

        // Now the recommended width should be 21
        assert_eq!(term.recommended_width(), 21);

        // Verify that we can still measure correctly with different content distributions
        // across scrollback and visible areas
        let surface_height = term.surface.dimensions().1;
        let scrollback_count = term.state.scrollback.len();

        // Ensure we actually have content in both scrollback and visible areas
        assert!(scrollback_count > 0, "Should have scrollback content");
        assert!(surface_height > 0, "Should have visible content");
    }

    #[test]
    fn test_long_lines_with_scroll_no_merge_and_correct_width() {
        // width=8, height=7; two long logical lines that will soft-wrap
        let mut term = Terminal::new(Options {
            cols: Some(8),
            rows: Some(7),
            background: None,
            foreground: None,
            env: HashMap::new(),
        });

        let s1: String = "A".repeat(17); // 17 columns
        let s2: String = "B".repeat(18); // 18 columns
        let input = format!("{}\n{}\n", s1, s2);

        let mut reader = std::io::Cursor::new(input.into_bytes());
        let mut writer = Vec::new();
        term.feed(&mut reader, &mut writer).unwrap();

        // Reconstruct logical lines by joining rows while the previous row was wrapped.
        let mut logicals: Vec<String> = Vec::new();
        let mut acc = String::new();
        let mut prev_wrapped = false;

        for cow in term.surface().screen_lines() {
            let line = cow;
            let mut text = String::new();
            for cell in line.visible_cells() {
                text.push_str(cell.str());
            }
            let trimmed = text.trim_end();

            if !acc.is_empty() && !prev_wrapped {
                logicals.push(acc);
                acc = String::new();
            }
            acc.push_str(trimmed);
            prev_wrapped = line.last_cell_was_wrapped();
        }
        if !acc.is_empty() {
            logicals.push(acc);
        }

        // We expect at least the last two logical lines to be the ones we input,
        // and they must not have been merged together.
        assert!(
            logicals.len() >= 2,
            "expected at least two logical lines after wrapping"
        );
        let n = logicals.len();
        assert_eq!(
            logicals[n - 2].len(),
            17,
            "first long logical line length mismatch"
        );
        assert_eq!(
            logicals[n - 1].len(),
            18,
            "second long logical line length mismatch"
        );
        assert!(
            logicals[n - 2].chars().all(|c| c == 'A'),
            "first logical line content corrupted"
        );
        assert!(
            logicals[n - 1].chars().all(|c| c == 'B'),
            "second logical line content corrupted"
        );

        // The recommended width should match the longest logical line
        assert_eq!(term.recommended_width(), 18);
    }

    #[test]
    fn test_many_long_lines_scroll_no_corruption() {
        // width=8, height=5; produce many long lines to force multiple scrolls.
        let mut term = Terminal::new(Options {
            cols: Some(8),
            rows: Some(5),
            background: None,
            foreground: None,
            env: HashMap::new(),
        });

        // Generate 12 lines alternating characters to detect any cross-line merging.
        let mut input = String::new();
        for i in 0..12 {
            let ch = if i % 2 == 0 { 'X' } else { 'Y' };
            let line: String = ch.to_string().repeat(13); // each 13 cols
            input.push_str(&line);
            input.push('\n');
        }

        let mut reader = std::io::Cursor::new(input.into_bytes());
        let mut writer = Vec::new();
        term.feed(&mut reader, &mut writer).unwrap();

        // Reconstruct logical lines
        let mut logicals: Vec<String> = Vec::new();
        let mut acc = String::new();
        let mut prev_wrapped = false;

        for cow in term.surface().screen_lines() {
            let line = cow;
            let mut text = String::new();
            for cell in line.visible_cells() {
                text.push_str(cell.str());
            }
            let trimmed = text.trim_end();

            if !acc.is_empty() && !prev_wrapped {
                logicals.push(acc);
                acc = String::new();
            }
            acc.push_str(trimmed);
            prev_wrapped = line.last_cell_was_wrapped();
        }
        if !acc.is_empty() {
            logicals.push(acc);
        }

        assert!(
            !logicals.is_empty(),
            "expected at least one logical line after feeding many lines"
        );

        // Check that the last few logical lines are intact and not merged with each other.
        let k = logicals.len().min(5);
        for j in 0..k {
            let s = &logicals[logicals.len() - 1 - j];
            assert!(
                s.chars().all(|c| c == 'X') || s.chars().all(|c| c == 'Y'),
                "logical line contains mixed characters (corruption): {:?}",
                s
            );
            assert_eq!(
                s.len(),
                13,
                "logical line length should be 13 after join across wraps"
            );
        }

        // The recommended width should match the longest logical line
        assert_eq!(term.recommended_width(), 13);
    }

    #[test]
    fn test_ledger_rotates_on_lf_at_bottom() {
        // width=4, height=2; write enough to reach bottom, then LF to cause scroll
        let mut term = Terminal::new(Options {
            cols: Some(4),
            rows: Some(2),
            background: None,
            foreground: None,
            env: HashMap::new(),
        });

        // "abcdef" wraps into bottom; "\n" triggers scroll from bottom
        let mut reader = Cursor::new("abcdef\n".as_bytes());
        let mut writer = Vec::new();
        term.feed(&mut reader, &mut writer).unwrap();

        // After LF at bottom, the screen should have scrolled:
        // top row contains prior content, bottom is blank.
        let lines = term.surface().screen_lines();
        let top: String = lines[0]
            .visible_cells()
            .map(|c| c.str().to_string())
            .collect();
        let bot: String = lines[1]
            .visible_cells()
            .map(|c| c.str().to_string())
            .collect();

        assert!(
            !top.trim_end().is_empty(),
            "top row should contain scrolled content after LF at bottom"
        );
        assert!(
            bot.trim_end().is_empty(),
            "bottom row should be blank after scroll from LF"
        );
    }

    #[test]
    fn test_bottom_autowrap_printstring_marks_previous_row() {
        // width=3, height=2 to force bottom autowrap within a single PrintString
        let mut term = Terminal::new(Options {
            cols: Some(3),
            rows: Some(2),
            background: None,
            foreground: None,
            env: HashMap::new(),
        });

        let mut reader = Cursor::new(b"abcdefg".as_ref());
        let mut writer = Vec::new();
        term.feed(&mut reader, &mut writer).unwrap();

        let lines = term.surface().screen_lines();
        // The row that wrapped before the bottom scroll should now be row 0 and be marked wrapped
        assert!(
            lines[0].last_cell_was_wrapped(),
            "previous bottom row (now row 0) should be marked as soft-wrapped after bottom autowrap"
        );
    }
    #[test]
    fn test_unscroll_rewrap_height_minimal_small_width() {
        // Minimal small-width repro: initial 8x2, three logical lines of length 9 each.
        // They will scroll out during feed, then we unscroll+rewrap and ensure height is 3.
        let mut term = Terminal::new(Options {
            cols: Some(8),
            rows: Some(2),
            background: None,
            foreground: None,
            env: HashMap::new(),
        });

        let data = "AAAAAAAAA\nBBBBBBBBB\nCCCCCCCCC\n";
        let mut reader = std::io::Cursor::new(data.as_bytes());
        let mut writer = Vec::new();
        term.feed(&mut reader, &mut writer).unwrap();

        // Full transcript width should be 9
        assert_eq!(term.recommended_width(), 9);

        // After rewrap to width=9, all three lines fit as single rows
        term.set_width(9);
        assert_eq!(term.recommended_height(), 3);
    }

    #[test]
    fn test_building_blocks_reusability() {
        // Test that our building blocks work correctly and can be reused for different computations
        let mut term = Terminal::new(Options {
            cols: Some(6),
            rows: Some(2),
            background: None,
            foreground: None,
            env: HashMap::new(),
        });

        // Add some content: "hello\n" + "verylongline\n" + "short"
        let mut reader = Cursor::new(b"hello\nverylongline\nshort".as_ref());
        let mut writer = Vec::new();
        term.feed(&mut reader, &mut writer).unwrap();

        // Test different uses of the same building block
        let recommended_width = term.recommended_width();

        // Demonstrate reusability: count logical lines
        let logical_line_count = term.process_logical_lines_with_accumulator(0, |count, _width| {
            *count += 1;
        });

        // Demonstrate reusability: compute total width
        let total_width = term.process_logical_lines_with_accumulator(0, |total, width| {
            *total += width;
        });

        // Verify expected values
        assert_eq!(
            recommended_width, 12,
            "longest line should be 'verylongline' (12 chars)"
        );
        assert_eq!(logical_line_count, 3, "should have 3 logical lines");
        assert_eq!(
            total_width,
            5 + 12 + 5,
            "total should be sum of all logical line widths"
        );

        // Verify that our building block produces the same recommended_width as the original would
        // by checking it matches the longest individual logical line
        let mut max_individual = 0;
        term.process_logical_lines_with_accumulator((), |_acc, width| {
            if width > max_individual {
                max_individual = width;
            }
        });
        assert_eq!(recommended_width as usize, max_individual);
    }

    #[test]
    fn test_unscroll_on_height_increase_minimal_small_width() {
        // Start 8x2; after set_width(9) we still see only bottom 2 lines.
        // Increasing height to 3 must unscroll the earliest line into view.
        let mut term = Terminal::new(Options {
            cols: Some(8),
            rows: Some(2),
            background: None,
            foreground: None,
            env: HashMap::new(),
        });

        let data = "AAAAAAAAA\nBBBBBBBBB\nCCCCCCCCC\n";
        let mut reader = std::io::Cursor::new(data.as_bytes());
        let mut writer = Vec::new();
        term.feed(&mut reader, &mut writer).unwrap();

        // Width across the full transcript is 9
        assert_eq!(term.recommended_width(), 9);

        // Change width only; keep height at 2 (bottom window shows last two lines)
        term.set_width(9);
        assert_eq!(term.surface().dimensions().1 as u16, 2);

        // Now increase height to 3; this should unscroll one more row into the visible window
        term.set_height(3);
        assert_eq!(term.surface().dimensions().1 as u16, 3);

        // Verify the top visible row is now the first logical line
        let lines = term.surface().screen_lines();
        let top: String = lines[0]
            .visible_cells()
            .map(|c| c.str().to_string())
            .collect();
        assert_eq!(top.trim_end(), "AAAAAAAAA");
    }
}
