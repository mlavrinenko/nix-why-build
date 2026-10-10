//! The progress line on a terminal's stderr: rewritten in place as the work
//! moves on, erased before anything else prints.

use std::io::Write;

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
    use super::{WIDTH, fit, frame};

    #[test]
    fn a_frame_rewinds_draws_and_erases_the_rest() {
        assert_eq!(frame("evaluating .#tank"), "\revaluating .#tank\x1b[K");
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
}
