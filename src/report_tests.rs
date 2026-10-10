use super::{Cascade, Package, Report, Root, Unknown};

fn package(name: &str, pname: Option<&str>) -> Package {
    Package {
        name: name.to_owned(),
        pname: pname.map(str::to_owned),
        version: None,
        drv: format!("/nix/store/x-{name}.drv"),
    }
}

fn cascade(name: &str, pname: Option<&str>) -> Cascade {
    Cascade {
        name: name.to_owned(),
        pname: pname.map(str::to_owned),
        drv: format!("/nix/store/x-{name}.drv"),
        chain: vec![name.to_owned()],
    }
}

fn root(name: &str, pname: Option<&str>, fixed_output: bool, cascades: Vec<Cascade>) -> Root {
    Root {
        name: name.to_owned(),
        pname: pname.map(str::to_owned),
        version: None,
        drv: format!("/nix/store/x-{name}.drv"),
        fixed_output,
        cascades,
    }
}

fn report(roots: Vec<Root>, to_build: Vec<Package>, unknown: Vec<Unknown>) -> Report {
    Report {
        toplevel: "/nix/store/x-system.drv".to_owned(),
        substituters: vec![],
        roots,
        to_build,
        unknown,
    }
}

#[test]
fn a_root_lists_its_named_cascades_and_caps_them_at_six() {
    let cascades = (1..=8)
        .map(|n| cascade(&format!("pkg-{n}"), Some("pkg")))
        .chain([cascade("etc", None)])
        .collect();
    let report = report(
        vec![root("src.tar.gz", None, true, cascades)],
        vec![],
        vec![],
    );
    assert_eq!(
        report.human(),
        "src.tar.gz (fetch)\n  drags in pkg-1, pkg-2, pkg-3, pkg-4, pkg-5, pkg-6 (+2 more)\n"
    );
}

#[test]
fn six_cascades_print_without_a_remainder() {
    let cascades = (1..=6)
        .map(|n| cascade(&format!("pkg-{n}"), Some("pkg")))
        .collect();
    let report = report(
        vec![root("zlib-1.3", Some("zlib"), false, cascades)],
        vec![],
        vec![],
    );
    assert_eq!(
        report.human(),
        "zlib-1.3\n  drags in pkg-1, pkg-2, pkg-3, pkg-4, pkg-5, pkg-6\n"
    );
}

#[test]
fn a_glue_root_with_no_named_cascade_is_not_printed_or_counted() {
    let report = report(
        vec![
            root("unit-x.service", None, false, vec![cascade("system", None)]),
            root("app-1.0", Some("app"), false, vec![]),
        ],
        vec![
            package("app-1.0", Some("app")),
            package("unit-x.service", None),
            package("system", None),
        ],
        vec![],
    );
    assert_eq!(report.human(), "app-1.0\n");
    assert_eq!(
        report.summary(),
        "1 root, 1 package to build, 2 more without a pname\n"
    );
}

#[test]
fn summary_names_the_first_unknown() {
    let report = report(
        vec![],
        vec![],
        vec![
            Unknown {
                drv: "/nix/store/a-x.drv".to_owned(),
                reason: "down".to_owned(),
            },
            Unknown {
                drv: "/nix/store/b-y.drv".to_owned(),
                reason: "gone".to_owned(),
            },
        ],
    );
    assert_eq!(
        report.summary(),
        "0 roots, 0 packages to build, 0 more without a pname, 2 unknown (first: /nix/store/a-x.drv: down)\n"
    );
}

#[test]
fn roots_dragging_in_the_same_packages_print_as_one_group() {
    let owner = || {
        vec![
            cascade("kache-1.0.0", Some("kache")),
            cascade("vendor", None),
        ]
    };
    let report = report(
        vec![
            root("arrayvec-0.7.8", Some("arrayvec"), false, owner()),
            root("zlib-1.3", Some("zlib"), false, vec![]),
            root("axum-0.8.9", Some("axum"), false, owner()),
            root("anyhow-1.0", Some("anyhow"), false, owner()),
            root("base64-0.23.1", Some("base64"), false, owner()),
        ],
        vec![],
        vec![],
    );
    assert_eq!(
        report.human(),
        "4 roots: anyhow-1.0, arrayvec-0.7.8, axum-0.8.9 (+1 more)\n  drags in kache-1.0.0\nzlib-1.3\n"
    );
    assert_eq!(
        report.summary(),
        "5 roots in 2 groups, 0 packages to build, 0 more without a pname\n"
    );
}

#[test]
fn a_group_of_fetches_says_so_and_needs_no_remainder_at_three() {
    let owner = || vec![cascade("corefonts-1", Some("corefonts"))];
    let report = report(
        vec![
            root("arial32.exe", None, true, owner()),
            root("andale32.exe", None, true, owner()),
            root("times32.exe", None, true, owner()),
        ],
        vec![],
        vec![],
    );
    assert_eq!(
        report.human(),
        "3 fetches: andale32.exe, arial32.exe, times32.exe\n  drags in corefonts-1\n"
    );
}

#[test]
fn standalone_roots_never_group() {
    let report = report(
        vec![
            root("nh-4.4.2", Some("nh"), false, vec![]),
            root("llama-cpp-0.5.0", Some("llama-cpp"), false, vec![]),
        ],
        vec![],
        vec![],
    );
    assert_eq!(report.human(), "llama-cpp-0.5.0\nnh-4.4.2\n");
}

#[test]
fn roots_dragging_in_different_packages_stay_apart() {
    let report = report(
        vec![
            root("a-src.tar.gz", None, true, vec![cascade("a-1", Some("a"))]),
            root("b-src.tar.gz", None, true, vec![cascade("b-1", Some("b"))]),
        ],
        vec![],
        vec![],
    );
    assert_eq!(
        report.human(),
        "a-src.tar.gz (fetch)\n  drags in a-1\nb-src.tar.gz (fetch)\n  drags in b-1\n"
    );
}
