use crate::reason::Reason;
use crate::report::{Cascade, Report, Root};

const BOLD: &str = "\x1b[1m";
const DIM: &str = "\x1b[2m";
const RESET: &str = "\x1b[0m";

fn root(name: &str, reason: Reason) -> Root {
    Root {
        name: name.to_owned(),
        pname: Some(name.to_owned()),
        version: None,
        drv: format!("/nix/store/x-{name}.drv"),
        fixed_output: false,
        cascades: vec![],
        requested_by: vec![],
        reason,
    }
}

/// A named cascade, so a root drags something in and a group can form.
fn lib() -> Cascade {
    Cascade {
        name: "lib-1".to_owned(),
        pname: Some("lib".to_owned()),
        drv: "/nix/store/x-lib-1.drv".to_owned(),
        chain: vec!["lib-1".to_owned()],
    }
}

fn report(roots: Vec<Root>) -> Report {
    Report {
        toplevel: "/nix/store/x-system.drv".to_owned(),
        substituters: vec![],
        roots,
        to_build: vec![],
        unknown: vec![],
        warnings: vec![],
    }
}

fn unknown(because: &str) -> Reason {
    Reason::Unknown {
        because: because.to_owned(),
    }
}

/// `text` without its SGR escape codes.
fn stripped(text: &str) -> String {
    let mut plain = String::new();
    let mut chars = text.chars();
    while let Some(ch) = chars.next() {
        if ch == '\x1b' {
            chars.by_ref().find(|end| *end == 'm');
        } else {
            plain.push(ch);
        }
    }
    plain
}

#[test]
fn a_styled_root_name_is_bold() {
    let report = report(vec![root("app-1.0", unknown("test"))]);
    assert!(
        report
            .human_styled()
            .starts_with(&format!("{BOLD}app-1.0{RESET}\n")),
        "{:?}",
        report.human_styled()
    );
}

#[test]
fn styled_labels_are_dim() {
    let mut app = root("app-1.0", unknown("test"));
    app.cascades = vec![lib()];
    let styled = report(vec![app]).human_styled();
    assert!(styled.contains(&format!("{DIM}why{RESET}")), "{styled:?}");
    assert!(
        styled.contains(&format!("{DIM}drags in{RESET}")),
        "{styled:?}"
    );
}

#[test]
fn the_plain_report_has_no_escape_code() {
    let report = report(vec![root("app-1.0", unknown("test"))]);
    assert_eq!(report.human(), "app-1.0\n  why unknown: test\n");
}

#[test]
fn styling_only_adds_escape_codes() {
    let reasons = [
        Reason::LocalOnly,
        Reason::Fetch { urls: vec![] },
        Reason::Unfree {
            against: "nixpkgs#a".to_owned(),
        },
        unknown("x"),
    ];
    let roots = reasons
        .into_iter()
        .enumerate()
        .map(|(at, reason)| root(&format!("pkg-{at}"), reason))
        .collect();
    let report = report(roots);
    assert_ne!(report.human_styled(), report.human());
    assert_eq!(stripped(&report.human_styled()), report.human());
}

#[test]
fn a_group_that_counts_its_categories_colors_each_one() {
    let shared = lib();
    let mut first = root("a-1", unknown("x"));
    let mut second = root("b-1", Reason::LocalOnly);
    first.cascades = vec![shared.clone()];
    second.cascades = vec![shared];
    let styled = report(vec![first, second]).human_styled();
    assert!(
        styled.contains(&format!("{DIM}local-only{RESET} (1), ")),
        "{styled:?}"
    );
}
