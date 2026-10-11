//! `scripts/wt-carry-lcov.sh` copies the worktree's coverage lcov to the primary
//! checkout when the worktrunk `pre-remove` hook fires, so `just crap` there
//! scores a landed merge without a second instrumented build. Each test builds
//! a primary checkout and a worktree of it, runs the script from the worktree
//! root as the hook does, and reads what reached the primary.
//!
//! Unix only: the script needs `bash`, `git` and `awk`.
#![cfg(unix)]

use std::path::{Path, PathBuf};
use std::process::{Command, Output};
use std::time::{Duration, SystemTime};

const SCRIPT: &str = "scripts/wt-carry-lcov.sh";
const STALE_SCRIPT: &str = "scripts/crap.sh";
const LCOV: &str = "target/coverage/lcov.info";
const SOURCE: &str = "src/lib.rs";

fn checkout() -> PathBuf {
    PathBuf::from(env!("CARGO_MANIFEST_DIR"))
}

struct Pair {
    dir: tempfile::TempDir,
}

impl Pair {
    /// A primary checkout on `main` and a worktree on `feature` at the same
    /// commit, both holding the two scripts, a source file and a test file.
    fn new() -> Self {
        let pair = Self {
            dir: tempfile::tempdir().expect("tempdir"),
        };
        let primary = pair.primary();
        std::fs::create_dir_all(&primary).expect("mkdir");
        git(&primary, &["init", "-q", "-b", "main"]);
        git(&primary, &["config", "user.email", "t@example.com"]);
        git(&primary, &["config", "user.name", "t"]);
        git(&primary, &["config", "commit.gpgsign", "false"]);
        for path in [SCRIPT, STALE_SCRIPT] {
            let text = std::fs::read_to_string(checkout().join(path)).expect("script");
            write(&primary.join(path), &text);
        }
        write(&primary.join(SOURCE), "pub fn a() {}\n");
        write(&primary.join("tests/a.rs"), "#[test]\nfn a() {}\n");
        commit(&primary, "base");
        let worktree = pair.worktree();
        let path = worktree.to_str().expect("utf-8 path");
        git(&primary, &["worktree", "add", "-q", "-b", "feature", path]);
        pair
    }

    fn primary(&self) -> PathBuf {
        self.dir.path().join("repo")
    }

    fn worktree(&self) -> PathBuf {
        self.dir.path().join("repo-feature")
    }

    /// Write the worktree's lcov as `cargo llvm-cov` does, with absolute `SF:`
    /// paths into the worktree, after the sources.
    fn instrument(&self) {
        let wt = self.worktree();
        let lcov = format!("TN:\nSF:{}/{SOURCE}\nDA:1,1\nend_of_record\n", wt.display());
        write(&wt.join(LCOV), &lcov);
        age(&wt.join(SOURCE), 3600);
        age(&wt.join("tests/a.rs"), 3600);
    }

    /// Run the script as the `pre-remove` hook does: from the worktree root,
    /// with the primary checkout as its argument.
    fn carry(&self, primary: &Path) -> Output {
        let out = Command::new("bash")
            .arg(SCRIPT)
            .arg(primary)
            .current_dir(self.worktree())
            .env_remove("BASH_ENV")
            .output()
            .expect("bash runs");
        assert!(out.status.success(), "{}", stderr(&out));
        out
    }

    fn carried(&self) -> Option<String> {
        std::fs::read_to_string(self.primary().join(LCOV)).ok()
    }
}

fn stderr(out: &Output) -> String {
    String::from_utf8_lossy(&out.stderr).into_owned()
}

fn git(dir: &Path, args: &[&str]) {
    let out = Command::new("git")
        .args(args)
        .current_dir(dir)
        .env("GIT_CONFIG_NOSYSTEM", "1")
        .output()
        .expect("git runs");
    assert!(out.status.success(), "git {args:?}:\n{}", stderr(&out));
}

fn write(path: &Path, content: &str) {
    std::fs::create_dir_all(path.parent().expect("parent")).expect("mkdir");
    std::fs::write(path, content).expect("write");
}

fn commit(dir: &Path, message: &str) {
    git(dir, &["add", "-A"]);
    git(dir, &["commit", "-q", "-m", message]);
}

/// Set `path`'s mtime to `secs` seconds in the past.
fn age(path: &Path, secs: u64) {
    let file = std::fs::File::options()
        .write(true)
        .open(path)
        .expect("open");
    file.set_modified(SystemTime::now() - Duration::from_secs(secs))
        .expect("set mtime");
}

#[test]
fn a_landed_merge_carries_the_lcov_with_its_paths_rewritten() {
    let pair = Pair::new();
    pair.instrument();
    pair.carry(&pair.primary());
    let carried = pair.carried().expect("the lcov reached the primary");
    let want = format!("SF:{}/{SOURCE}", pair.primary().display());
    assert!(carried.contains(&want), "SF path not rewritten:\n{carried}");
    assert!(!carried.contains(pair.worktree().to_str().expect("utf-8 path")));
}

#[test]
fn a_branch_that_did_not_land_carries_nothing() {
    let pair = Pair::new();
    pair.instrument();
    let wt = pair.worktree();
    write(&wt.join("src/other.rs"), "pub fn b() {}\n");
    commit(&wt, "work");
    pair.carry(&pair.primary());
    assert!(
        pair.carried().is_none(),
        "the lcov of an unlanded tree moved"
    );
}

#[test]
fn an_lcov_older_than_a_source_is_not_carried() {
    let pair = Pair::new();
    pair.instrument();
    age(&pair.worktree().join(LCOV), 7200);
    pair.carry(&pair.primary());
    assert!(pair.carried().is_none(), "a stale lcov was carried");
}

#[test]
fn a_newer_lcov_in_the_primary_is_kept() {
    let pair = Pair::new();
    pair.instrument();
    age(&pair.worktree().join(LCOV), 600);
    write(&pair.primary().join(LCOV), "primary's own\n");
    pair.carry(&pair.primary());
    assert_eq!(pair.carried().as_deref(), Some("primary's own\n"));
}

#[test]
fn a_worktree_without_an_lcov_is_a_no_op() {
    let pair = Pair::new();
    pair.carry(&pair.primary());
    assert!(pair.carried().is_none());
}

#[test]
fn a_failed_carry_says_so_and_never_fails_the_hook() {
    let pair = Pair::new();
    pair.instrument();
    // `target` is a file in the primary, so `target/coverage` cannot be made.
    write(&pair.primary().join("target"), "");
    let out = pair.carry(&pair.primary());
    assert!(stderr(&out).contains("could not carry"));
}

#[test]
fn a_primary_that_is_no_checkout_is_a_no_op() {
    let pair = Pair::new();
    pair.instrument();
    let missing = pair.dir.path().join("no-such-checkout");
    pair.carry(&missing);
    assert!(!missing.exists());
}
