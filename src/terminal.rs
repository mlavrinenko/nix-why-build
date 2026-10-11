//! The progress line on a terminal's stderr: rewritten in place as the work
//! moves on, erased before anything else prints.

use std::ffi::OsStr;
use std::io::Write;

use clap::ColorChoice;

/// The longest line drawn. A longer one would wrap, and `\r` only rewinds
/// the row the cursor is on.
const WIDTH: usize = 79;

/// Back to the start of the line, then erase it.
const CLEAR: &str = "\r\x1b[K";

/// `text` drawn over whatever the line held: carriage return, the text cut
/// to [`WIDTH`], erase the rest of the line.
fn frame(text: &str) -> String {
    format!("\r{}\x1b[K", fit(text))
}

/// `text`, or its first characters and an ellipsis when it is too long.
fn fit(text: &str) -> String {
    if text.chars().count() <= WIDTH {
        return text.to_owned();
    }
    let mut cut: String = text.chars().take(WIDTH - 1).collect();
    cut.push('…');
    cut
}

/// Whether the report gets colors. `always` and `never` are orders; `auto`
/// colors a terminal, unless `NO_COLOR` is set to anything but nothing
/// (<https://no-color.org>).
pub fn wants_color(
    choice: ColorChoice,
    no_color: Option<&OsStr>,
    stdout_is_terminal: bool,
) -> bool {
    match choice {
        ColorChoice::Always => true,
        ColorChoice::Never => false,
        ColorChoice::Auto => stdout_is_terminal && no_color.is_none_or(OsStr::is_empty),
    }
}

/// Draws `text` as the progress line.
pub fn draw(text: &str) {
    write_stderr(&frame(text));
}

/// Erases the progress line.
pub fn clear() {
    write_stderr(CLEAR);
}

/// A progress line is a courtesy: a stderr that cannot be written to is not
/// worth failing the run for.
fn write_stderr(text: &str) {
    let mut stderr = std::io::stderr().lock();
    let _ = stderr.write_all(text.as_bytes());
    let _ = stderr.flush();
}

#[cfg(test)]
mod tests {
    use std::ffi::OsStr;

    use clap::ColorChoice::{Always, Auto, Never};

    use super::{WIDTH, fit, frame, wants_color};

    #[test]
    fn a_frame_rewinds_draws_and_erases_the_rest() {
        assert_eq!(frame("evaluating .#mypc"), "\revaluating .#mypc\x1b[K");
    }

    #[test]
    fn a_line_that_fits_is_kept() {
        let exact = "x".repeat(WIDTH);
        assert_eq!(fit(&exact), exact);
    }

    #[test]
    fn a_line_that_would_wrap_is_cut_with_an_ellipsis() {
        let cut = fit(&"x".repeat(WIDTH + 1));
        assert_eq!(cut.chars().count(), WIDTH);
        assert!(cut.ends_with('…'));
    }

    #[test]
    fn cutting_counts_characters_not_bytes() {
        let cut = fit(&"é".repeat(WIDTH + 5));
        assert_eq!(cut.chars().count(), WIDTH);
    }

    #[test]
    fn auto_colors_a_terminal() {
        assert!(wants_color(Auto, None, true));
        assert!(!wants_color(Auto, None, false));
    }

    #[test]
    fn auto_yields_to_no_color_unless_it_is_empty() {
        assert!(!wants_color(Auto, Some(OsStr::new("1")), true));
        assert!(wants_color(Auto, Some(OsStr::new("")), true));
    }

    #[test]
    fn always_and_never_ignore_the_environment() {
        assert!(wants_color(Always, Some(OsStr::new("1")), false));
        assert!(!wants_color(Never, None, true));
    }
}
