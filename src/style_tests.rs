use super::Styles;
use crate::diff::Diff;
use crate::reason::Reason;

fn every_category() -> Vec<Reason> {
    let against = "nixpkgs#a".to_owned();
    vec![
        Reason::LocalOnly,
        Reason::Fetch { urls: vec![] },
        Reason::Overridden {
            against: against.clone(),
            against_cached: true,
            diff: Diff::default(),
        },
        Reason::Unfree {
            against: against.clone(),
        },
        Reason::NotBuiltByHydra {
            against: against.clone(),
            because: "meta.broken".to_owned(),
        },
        Reason::NewerThanCache { against },
        Reason::NoCounterpart { tried: vec![] },
        Reason::Unknown {
            because: "x".to_owned(),
        },
    ]
}

#[test]
fn the_plain_palette_prints_text_as_is() {
    let plain = Styles::plain();
    assert_eq!(plain.root("app-1.0"), "app-1.0");
    assert_eq!(plain.label("why"), "why");
    for reason in every_category() {
        assert_eq!(plain.category(&reason), reason.category());
    }
}

#[test]
fn every_category_has_its_own_look() {
    let terminal = Styles::terminal();
    let mut looks: Vec<&str> = Vec::new();
    let painted: Vec<(String, &str)> = every_category()
        .iter()
        .map(|reason| (terminal.category(reason), reason.category()))
        .collect();
    for (look, category) in &painted {
        assert!(look.ends_with(&format!("{category}\x1b[0m")), "{look:?}");
        looks.push(look.split(category).next().unwrap_or_default());
    }
    assert!(
        looks.iter().all(|look| look.starts_with('\x1b')),
        "{looks:?}"
    );
    // Fetch and local-only recede alike; the other six differ.
    looks.sort_unstable();
    looks.dedup();
    assert_eq!(looks.len(), 7);
}

#[test]
fn overridden_and_unknown_stand_out_and_fetch_and_local_only_recede() {
    let terminal = Styles::terminal();
    let paint = |category: &str| {
        let reason = every_category()
            .into_iter()
            .find(|reason| reason.category() == category);
        reason
            .map(|reason| terminal.category(&reason))
            .unwrap_or_default()
    };
    assert_eq!(paint("overridden"), "\x1b[1m\x1b[33moverridden\x1b[0m");
    assert_eq!(paint("unknown"), "\x1b[1m\x1b[31munknown\x1b[0m");
    assert_eq!(paint("fetch"), "\x1b[2mfetch\x1b[0m");
    assert_eq!(paint("local-only"), "\x1b[2mlocal-only\x1b[0m");
}
