//! How the human report is colored: bold root names, dim labels, one color
//! per category. The plain palette is made of empty styles, which render as
//! nothing, so one code path prints both.

use anstyle::{AnsiColor, Effects, Style};

use crate::reason::Reason;

/// The styles a report is printed with.
#[derive(Clone, Copy, Debug)]
pub(crate) struct Styles {
    colored: bool,
}

impl Styles {
    /// Every style empty: the text comes out as is.
    pub(crate) fn plain() -> Self {
        Self { colored: false }
    }

    /// For a terminal that reads ANSI escape codes.
    pub(crate) fn terminal() -> Self {
        Self { colored: true }
    }

    /// A root's name.
    pub(crate) fn root(self, name: &str) -> String {
        self.paint(Style::new().effects(Effects::BOLD), name)
    }

    /// A line's label: `why`, `wanted by`, `drags in`.
    pub(crate) fn label(self, label: &str) -> String {
        self.paint(Style::new().effects(Effects::DIMMED), label)
    }

    /// The category of `reason`.
    pub(crate) fn category(self, reason: &Reason) -> String {
        self.paint(category_style(reason), reason.category())
    }

    fn paint(self, style: Style, text: &str) -> String {
        if !self.colored {
            return text.to_owned();
        }
        format!("{}{text}{}", style.render(), style.render_reset())
    }
}

/// What to act on stands out (`overridden`, `unknown`), what no cache could
/// fix recedes (`fetch`, `local-only`), the rest take a color each.
fn category_style(reason: &Reason) -> Style {
    let bold = Style::new().effects(Effects::BOLD);
    match reason {
        Reason::Overridden { .. } => bold.fg_color(Some(AnsiColor::Yellow.into())),
        Reason::Unknown { .. } => bold.fg_color(Some(AnsiColor::Red.into())),
        Reason::Unfree { .. } => Style::new().fg_color(Some(AnsiColor::Magenta.into())),
        Reason::NotBuiltByHydra { .. } => Style::new().fg_color(Some(AnsiColor::Blue.into())),
        Reason::NewerThanCache { .. } => Style::new().fg_color(Some(AnsiColor::Green.into())),
        Reason::NoCounterpart { .. } => Style::new().fg_color(Some(AnsiColor::Cyan.into())),
        Reason::Fetch { .. } | Reason::Vendored | Reason::LocalOnly => {
            Style::new().effects(Effects::DIMMED)
        }
    }
}

#[cfg(test)]
#[path = "style_tests.rs"]
mod tests;
