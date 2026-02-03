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
fn test_print_single_char_bottom_scroll() {
    // Test that Print action with a single character causing bottom scroll
    // correctly captures scrollback (covers the Print branch in scrollback capture)
    // Feed characters one at a time to avoid consolidation into PrintString
    let mut term = Terminal::new(Options {
        cols: Some(4),
        rows: Some(2),
        background: None,
        foreground: None,
        env: HashMap::new(),
    });

    let mut writer = Vec::new();

    // Fill first row with single character feeds
    for ch in ['A', 'B', 'C', 'D'] {
        let mut reader = Cursor::new(vec![ch as u8]);
        term.feed(&mut reader, &mut writer).unwrap();
    }

    // Move to next row with newline
    let mut reader = Cursor::new(b"\n");
    term.feed(&mut reader, &mut writer).unwrap();

    // Fill most of second row (bottom row)
    for ch in ['E', 'F', 'G'] {
        let mut reader = Cursor::new(vec![ch as u8]);
        term.feed(&mut reader, &mut writer).unwrap();
    }

    // Now at position (3, 1), 1 column left
    // Feed a wide character that won't fit (triggers wrap and scroll)
    let mut reader = Cursor::new("🔥".as_bytes());
    term.feed(&mut reader, &mut writer).unwrap();

    // Should have captured row 0 ("ABCD") in scrollback
    assert_eq!(
        term.state.scrollback.len(),
        1,
        "Should have one line in scrollback"
    );

    let scrollback_line = &term.state.scrollback[0];
    let text: String = scrollback_line
        .visible_cells()
        .map(|c| c.str().to_string())
        .collect();
    assert_eq!(text.trim(), "ABCD", "Scrollback should contain 'ABCD'");
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
    assert_eq!(term.surface().dimensions().1, 2);

    // Now increase height to 3; this should unscroll one more row into the visible window
    term.set_height(3);
    assert_eq!(term.surface().dimensions().1, 3);

    // Verify the top visible row is now the first logical line
    let lines = term.surface().screen_lines();
    let top: String = lines[0]
        .visible_cells()
        .map(|c| c.str().to_string())
        .collect();
    assert_eq!(top.trim_end(), "AAAAAAAAA");
}

#[test]
fn test_wrap_flags_edge_case_empty() {
    // Test when wrap_flags might be in edge state
    let mut term = Terminal::new(Options {
        cols: Some(3),
        rows: Some(1),
        background: None,
        foreground: None,
        env: HashMap::new(),
    });

    let mut writer = Vec::new();

    // With height 1, create a scenario that tests wrap flag rotation
    let mut reader = Cursor::new(b"abc");
    term.feed(&mut reader, &mut writer).unwrap();

    // Force a scroll with more content
    let mut reader = Cursor::new(b"def");
    term.feed(&mut reader, &mut writer).unwrap();

    // Verify terminal state is consistent
    assert_eq!(term.surface().dimensions().0, 3);
    assert_eq!(term.surface().dimensions().1, 1);
}

#[test]
fn test_printstring_very_wide_character_breaks_loop() {
    let mut term = Terminal::new(Options {
        cols: Some(1),
        rows: Some(2),
        background: None,
        foreground: None,
        env: HashMap::new(),
    });

    let mut writer = Vec::new();

    // Fill first row to move to bottom
    let mut reader = Cursor::new(b"a");
    term.feed(&mut reader, &mut writer).unwrap();

    // Move to bottom row
    let mut reader = Cursor::new(b"\n");
    term.feed(&mut reader, &mut writer).unwrap();

    // Now at bottom with no space left - feed PrintString with emoji
    // This should trigger multiple wraps and the chunk_chars == 0 safety break
    let mut reader = Cursor::new("🔥🔥🔥".as_bytes());
    term.feed(&mut reader, &mut writer).unwrap();

    // Verify terminal doesn't crash and maintains consistent state
    let (w, h) = term.surface().dimensions();
    assert_eq!(w, 1);
    assert_eq!(h, 2);
}

#[test]
fn test_print_wrap_within_buffer() {
    let mut term = Terminal::new(Options {
        cols: Some(4),
        rows: Some(3),
        background: None,
        foreground: None,
        env: HashMap::new(),
    });

    let mut writer = Vec::new();

    // Fill first row completely
    let mut reader = Cursor::new(b"AAAA");
    term.feed(&mut reader, &mut writer).unwrap();

    // Now feed a wide character that won't fit, triggering wrap within buffer (row 0 -> row 1)
    // We're at row 0, position (4,0), and feed a 2-width emoji
    let mut reader = Cursor::new("🔥".as_bytes());
    term.feed(&mut reader, &mut writer).unwrap();

    // Verify wrap occurred and we're not at bottom (still have room)
    let lines = term.surface().screen_lines();
    assert!(
        lines[0].last_cell_was_wrapped(),
        "Row 0 should be marked as wrapped"
    );

    // Verify content layout
    let r0: String = lines[0]
        .visible_cells()
        .map(|c| c.str().to_string())
        .collect();
    assert_eq!(
        r0.trim_end(),
        "AAAA",
        "Row 0 should contain original content"
    );
}

fn make_term(cols: u16, rows: u16) -> Terminal {
    Terminal::new(Options {
        cols: Some(cols),
        rows: Some(rows),
        background: None,
        foreground: None,
        env: HashMap::new(),
    })
}

fn feed(term: &mut Terminal, data: &[u8]) {
    term.feed(Cursor::new(data), &mut Vec::new()).unwrap();
}

fn visible_line_text(term: &Terminal, row: usize) -> String {
    term.surface().screen_lines()[row]
        .visible_cells()
        .map(|c| c.str().to_string())
        .collect()
}

#[test]
fn test_show_command_in_surface() {
    let mut term = make_term(80, 5);

    feed(&mut term, b"$ \x1b[1mgit status -s\x1b[0m\n");
    feed(&mut term, b" M src/main.rs\n");

    let line0 = visible_line_text(&term, 0);
    assert!(line0.contains("$ "), "line 0 missing prompt: {line0:?}");
    assert!(
        line0.contains("git status -s"),
        "line 0 missing command: {line0:?}"
    );

    let line1 = visible_line_text(&term, 1);
    assert!(
        line1.contains("M src/main.rs"),
        "line 1 missing output: {line1:?}"
    );
}

#[test]
fn test_show_command_with_special_chars() {
    let mut term = make_term(80, 3);

    let cmd_line = crate::command::to_terminal("$ ", "echo", &["Hello, World!".to_string()], None);
    feed(&mut term, &cmd_line);

    let line0 = visible_line_text(&term, 0);
    assert!(line0.contains("$ "), "line 0 missing prompt: {line0:?}");
    assert!(line0.contains("echo"), "line 0 missing command: {line0:?}");
}
