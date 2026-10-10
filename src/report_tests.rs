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
        requested_by: vec![],
        reason: super::Reason::Unknown {
            because: "test".to_owned(),
        },
    }
}

fn report(roots: Vec<Root>, to_build: Vec<Package>, unknown: Vec<Unknown>) -> Report {
    Report {
        toplevel: "/nix/store/x-system.drv".to_owned(),
        substituters: vec![],
        roots,
        to_build,
        unknown,
        warnings: vec![],
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
        "src.tar.gz (fetch)\n  why unknown: test\n  drags in pkg-1, pkg-2, pkg-3, pkg-4, pkg-5, pkg-6 (+2 more)\n"
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
        "zlib-1.3\n  why unknown: test\n  drags in pkg-1, pkg-2, pkg-3, pkg-4, pkg-5, pkg-6\n"
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
    assert_eq!(report.human(), "app-1.0\n  why unknown: test\n");
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
        "4 roots: anyhow-1.0, arrayvec-0.7.8, axum-0.8.9 (+1 more)\n  why unknown: test\n  drags in kache-1.0.0\nzlib-1.3\n  why unknown: test\n"
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
        "3 fetches: andale32.exe, arial32.exe, times32.exe\n  why unknown: test\n  drags in corefonts-1\n"
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
    assert_eq!(
        report.human(),
        "llama-cpp-0.5.0\n  why unknown: test\nnh-4.4.2\n  why unknown: test\n"
    );
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
        "a-src.tar.gz (fetch)\n  why unknown: test\n  drags in a-1\nb-src.tar.gz (fetch)\n  why unknown: test\n  drags in b-1\n"
    );
}

fn wanted(root: Root, options: &[&str]) -> Root {
    Root {
        requested_by: options
            .iter()
            .map(|option| super::Requested {
                option: (*option).to_owned(),
                via: "glue".to_owned(),
                via_drv: "/nix/store/x-glue.drv".to_owned(),
                item: "item".to_owned(),
                item_drv: "/nix/store/x-item.drv".to_owned(),
                files: vec![],
            })
            .collect(),
        ..root
    }
}

#[test]
fn a_group_says_once_which_options_want_it_and_caps_them_at_two() {
    let owner = || vec![cascade("kache-1.0.0", Some("kache"))];
    let report = report(
        vec![
            wanted(
                root("a-1", Some("a"), false, owner()),
                &["environment.systemPackages", "systemd.services.b"],
            ),
            wanted(
                root("b-1", Some("b"), false, owner()),
                &[
                    "environment.systemPackages",
                    "home.packages",
                    "systemd.services.a",
                ],
            ),
        ],
        vec![],
        vec![],
    );
    assert_eq!(
        report.human(),
        "2 roots: a-1, b-1\n  why unknown: test\n  wanted by environment.systemPackages, home.packages (+2 more)\n  drags in kache-1.0.0\n"
    );
}

#[test]
fn toplevel_alone_is_not_worth_a_line_but_beside_an_option_it_is() {
    let report = report(
        vec![
            wanted(
                root("glibc-locales-2.42", Some("glibc-locales"), false, vec![]),
                &["toplevel"],
            ),
            wanted(
                root("nh-4.4.2", Some("nh"), false, vec![]),
                &["environment.systemPackages", "toplevel"],
            ),
        ],
        vec![],
        vec![],
    );
    assert_eq!(
        report.human(),
        "glibc-locales-2.42\n  why unknown: test\nnh-4.4.2\n  why unknown: test\n  wanted by environment.systemPackages, toplevel\n"
    );
}

#[test]
fn wanted_by_names_up_to_two_files_per_option() {
    let mut root = wanted(
        root("llama-cpp-0.5.0", Some("llama-cpp"), false, vec![]),
        &["environment.systemPackages", "systemd.services.llama"],
    );
    if let [packages, unit] = root.requested_by.as_mut_slice() {
        packages.files = vec!["apps/llama.nix".to_owned()];
        unit.files = vec![
            "apps/llama.nix".to_owned(),
            "base:apps/gpu.nix".to_owned(),
            "nixpkgs:x.nix".to_owned(),
        ];
    }
    let report = report(vec![root], vec![], vec![]);
    assert_eq!(
        report.human(),
        "llama-cpp-0.5.0\n  why unknown: test\n  wanted by environment.systemPackages (apps/llama.nix), systemd.services.llama (apps/llama.nix, base:apps/gpu.nix +1)\n"
    );
}

#[test]
fn warnings_print_before_the_tally() {
    let mut report = report(vec![], vec![], vec![]);
    report
        .warnings
        .push("module files unknown: error: boom".to_owned());
    assert_eq!(
        report.summary(),
        "warning: module files unknown: error: boom\n0 roots, 0 packages to build, 0 more without a pname\n"
    );
}

#[test]
fn package_lists_come_first_then_units_then_files() {
    let root = wanted(
        root("kache-1.0.0", Some("kache"), false, vec![]),
        &[
            "environment.etc.profile",
            "systemd.user.services.kache",
            "home-manager.users.tank.home.packages",
        ],
    );
    assert_eq!(
        report(vec![root], vec![], vec![]).human(),
        "kache-1.0.0\n  why unknown: test\n  wanted by home-manager.users.tank.home.packages, systemd.user.services.kache (+1 more)\n"
    );
}

#[test]
fn a_group_shares_its_reason_or_counts_categories() {
    let owner = || vec![cascade("octrimmer-0.1.1", Some("octrimmer"))];
    let with = |name: &str, reason: super::Reason| Root {
        reason,
        ..root(name, None, true, owner())
    };
    let fetch = |url: &str| super::Reason::Fetch {
        urls: vec![url.to_owned()],
    };
    let shared = report(
        vec![
            with("a.tgz", fetch("https://same")),
            with("b.tgz", fetch("https://same")),
        ],
        vec![],
        vec![],
    );
    assert_eq!(
        shared.human(),
        "2 fetches: a.tgz, b.tgz\n  why fetch: no cache has https://same\n  drags in octrimmer-0.1.1\n"
    );
    let mixed = report(
        vec![
            with("a.tgz", fetch("https://a")),
            with("b.tgz", fetch("https://b")),
            with("c", super::Reason::NoCounterpart { tried: vec![] }),
        ],
        vec![],
        vec![],
    );
    assert_eq!(
        mixed.human(),
        "3 fetches: a.tgz, b.tgz, c\n  why fetch (2), no-counterpart (1)\n  drags in octrimmer-0.1.1\n"
    );
}
