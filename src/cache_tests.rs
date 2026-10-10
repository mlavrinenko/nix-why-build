use std::collections::HashMap;

use nix_derivation::StorePath;
use std::sync::Arc;
use std::time::Duration;

use tokio::io::{AsyncReadExt, AsyncWriteExt};
use tokio::sync::Barrier;

use super::{Availability, Substituters};
use crate::fixture_tests::{PUBLIC, SECRET, file_cache, narinfo, narinfo_name};

const ROGUE: &str = "rogue-1:WSWrsHVFjRnBaZytl7Cdh0eHb6zbSz9y24G6xtmQbVWvi0KYP2v7rj4+gTTXRbYWWsryCjAF3oGDOq3FTlJeEA==";
/// Claims the trusted key's name, signs with another secret.
const FORGED: &str = "test-cache-1:lt2k779oTCU/mF2pbGitQK2tcmjqAMhi9KRBA5s7Dkg54t8HZgNEyhYbmGfTmyfSgLV6C6yQZrm6zMjxsTGkUQ==";

fn store_path(seed: u8, name: &str) -> StorePath {
    StorePath::from_parts([seed; 20], name).expect("valid store path")
}

fn substituters(urls: &[String], require_sigs: bool) -> Substituters {
    Substituters::new(urls, &[PUBLIC.to_owned()], require_sigs).expect("valid substituters")
}

async fn ask(subs: &Substituters, path: &StorePath) -> Availability {
    let answers = subs.query(std::slice::from_ref(path)).await;
    answers.get(path).cloned().expect("an answer per path")
}

async fn http_cache(routes: HashMap<String, (u16, String)>) -> String {
    http_cache_gated(routes, None, vec![]).await
}

/// Serves `routes` (URL path to status and body) over HTTP/1.1, one request
/// per connection, and returns the base URL. Each response waits for `gate`
/// when given, then for the `delays` entry of its connection (the last
/// entry repeats; none means no delay).
async fn http_cache_gated(
    routes: HashMap<String, (u16, String)>,
    gate: Option<Arc<Barrier>>,
    delays: Vec<Duration>,
) -> String {
    let listener = tokio::net::TcpListener::bind("127.0.0.1:0")
        .await
        .expect("bind");
    let addr = listener.local_addr().expect("local addr");
    tokio::spawn(async move {
        let mut connection = 0_usize;
        while let Ok((mut socket, _)) = listener.accept().await {
            let delay = delays
                .get(connection)
                .or(delays.last())
                .copied()
                .unwrap_or_default();
            connection += 1;
            let mut buf = vec![0_u8; 4096];
            let read = socket.read(&mut buf).await.unwrap_or(0);
            let request = String::from_utf8_lossy(buf.get(..read).unwrap_or_default()).into_owned();
            let target = request.split_whitespace().nth(1).unwrap_or("/").to_owned();
            if let Some(gate) = &gate {
                gate.wait().await;
            }
            let (status, body) = routes.get(&target).cloned().unwrap_or((404, String::new()));
            let response = format!(
                "HTTP/1.1 {status} X\r\nContent-Length: {}\r\nConnection: close\r\n\r\n{body}",
                body.len()
            );
            tokio::spawn(async move {
                tokio::time::sleep(delay).await;
                let _ = socket.write_all(response.as_bytes()).await;
            });
        }
    });
    format!("http://{addr}")
}

#[tokio::test]
async fn a_trusted_narinfo_in_a_file_cache_is_served() {
    let dir = tempfile::tempdir().expect("tempdir");
    let path = store_path(1, "hello-2.12");
    let url = file_cache(dir.path(), &[(&path, narinfo(&path, Some(SECRET)))]);
    let subs = substituters(std::slice::from_ref(&url), true);
    assert_eq!(
        ask(&subs, &path).await,
        Availability::Served { substituter: url }
    );
}

#[tokio::test]
async fn an_absent_narinfo_is_missing() {
    let dir = tempfile::tempdir().expect("tempdir");
    let url = file_cache(dir.path(), &[]);
    let subs = substituters(&[url], true);
    assert_eq!(
        ask(&subs, &store_path(1, "hello-2.12")).await,
        Availability::Missing
    );
}

#[tokio::test]
async fn a_narinfo_signed_by_an_untrusted_key_is_missing() {
    let dir = tempfile::tempdir().expect("tempdir");
    let path = store_path(1, "hello-2.12");
    let url = file_cache(dir.path(), &[(&path, narinfo(&path, Some(ROGUE)))]);
    let subs = substituters(&[url], true);
    assert_eq!(ask(&subs, &path).await, Availability::Missing);
}

#[tokio::test]
async fn an_unsigned_narinfo_is_missing_while_signatures_are_required() {
    let dir = tempfile::tempdir().expect("tempdir");
    let path = store_path(1, "hello-2.12");
    let url = file_cache(dir.path(), &[(&path, narinfo(&path, None))]);
    let subs = substituters(&[url], true);
    assert_eq!(ask(&subs, &path).await, Availability::Missing);
}

#[tokio::test]
async fn an_unsigned_narinfo_is_served_once_signatures_are_off() {
    let dir = tempfile::tempdir().expect("tempdir");
    let path = store_path(1, "hello-2.12");
    let url = file_cache(dir.path(), &[(&path, narinfo(&path, None))]);
    let subs = substituters(std::slice::from_ref(&url), false);
    assert_eq!(
        ask(&subs, &path).await,
        Availability::Served { substituter: url }
    );
}

#[tokio::test]
async fn a_narinfo_for_another_path_is_unknown() {
    let dir = tempfile::tempdir().expect("tempdir");
    let path = store_path(1, "hello-2.12");
    let other = store_path(2, "hello-2.12");
    let url = file_cache(dir.path(), &[(&path, narinfo(&other, Some(SECRET)))]);
    let subs = substituters(&[url], true);
    assert!(matches!(
        ask(&subs, &path).await,
        Availability::Unknown { reason } if reason.contains("narinfo is for")
    ));
    assert_eq!(subs.warnings(), Vec::<String>::new());
}

#[tokio::test]
async fn the_first_substituter_that_serves_a_path_answers() {
    let empty = tempfile::tempdir().expect("tempdir");
    let full = tempfile::tempdir().expect("tempdir");
    let path = store_path(1, "hello-2.12");
    let first = file_cache(empty.path(), &[]);
    let second = file_cache(full.path(), &[(&path, narinfo(&path, Some(SECRET)))]);
    let subs = substituters(&[first, second.clone()], true);
    assert_eq!(
        ask(&subs, &path).await,
        Availability::Served {
            substituter: second
        }
    );
}

#[tokio::test]
async fn http_serves_a_trusted_narinfo_and_misses_on_404() {
    let served = store_path(1, "hello-2.12");
    let absent = store_path(2, "absent-1.0");
    let routes = HashMap::from([(
        format!("/{}", narinfo_name(&served)),
        (200, narinfo(&served, Some(SECRET))),
    )]);
    let url = http_cache(routes).await;
    let subs = substituters(std::slice::from_ref(&url), true);
    let answers = subs.query(&[served.clone(), absent.clone()]).await;
    assert_eq!(
        answers.get(&served),
        Some(&Availability::Served { substituter: url })
    );
    assert_eq!(answers.get(&absent), Some(&Availability::Missing));
}

#[tokio::test]
async fn an_http_server_error_is_unknown_and_names_the_status() {
    let path = store_path(1, "hello-2.12");
    let other = store_path(2, "hello-2.12");
    let routes = HashMap::from([
        (format!("/{}", narinfo_name(&path)), (503, String::new())),
        (format!("/{}", narinfo_name(&other)), (503, String::new())),
    ]);
    let url = http_cache(routes).await;
    let subs = substituters(std::slice::from_ref(&url), true);
    assert_eq!(
        ask(&subs, &path).await,
        Availability::Unknown {
            reason: format!("{url}: HTTP 503 Service Unavailable")
        }
    );
    assert_eq!(subs.warnings(), Vec::<String>::new());
    assert!(
        matches!(
            ask(&subs, &store_path(2, "hello-2.12")).await,
            Availability::Unknown { .. }
        ),
        "a server error does not disable the substituter"
    );
}

#[tokio::test]
async fn a_later_hit_beats_an_earlier_failure() {
    let full = tempfile::tempdir().expect("tempdir");
    let path = store_path(1, "hello-2.12");
    let served = file_cache(full.path(), &[(&path, narinfo(&path, Some(SECRET)))]);
    let broken = http_cache(HashMap::from([(
        format!("/{}", narinfo_name(&path)),
        (503, String::new()),
    )]))
    .await;
    let subs = substituters(&[broken, served.clone()], true);
    assert_eq!(
        ask(&subs, &path).await,
        Availability::Served {
            substituter: served
        }
    );
}

#[tokio::test]
async fn an_unsupported_scheme_is_left_out_with_one_warning() {
    let dir = tempfile::tempdir().expect("tempdir");
    let cache = file_cache(dir.path(), &[]);
    let urls = [
        "s3://bucket".to_owned(),
        cache.clone(),
        "s3://bucket".to_owned(),
        "ssh-ng://builder".to_owned(),
    ];
    let subs = substituters(&urls, true);
    assert_eq!(subs.urls(), vec![cache.as_str()]);
    assert_eq!(
        ask(&subs, &store_path(1, "hello-2.12")).await,
        Availability::Missing
    );
    assert_eq!(
        subs.warnings(),
        vec![
            "s3://bucket: unsupported scheme, not asked",
            "ssh-ng://builder: unsupported scheme, not asked"
        ]
    );
}

#[test]
fn a_repeated_substituter_is_asked_once_and_query_strings_are_ignored() {
    let urls = [
        "https://cache.nixos.org/".to_owned(),
        "https://cache.example?priority=40".to_owned(),
        "https://cache.nixos.org/".to_owned(),
    ];
    let subs = substituters(&urls, true);
    assert_eq!(
        subs.urls(),
        vec![
            "https://cache.nixos.org/",
            "https://cache.example?priority=40"
        ]
    );
    assert_eq!(
        format!("{:?}", subs.list.get(1).map(|entry| &entry.location)),
        r#"Some(Http("https://cache.example"))"#
    );
}

#[test]
fn a_malformed_trusted_key_is_refused_by_name() {
    let err = Substituters::new(&[], &["no-separator".to_owned()], true)
        .expect_err("a key without a colon");
    assert!(err.to_string().starts_with(r#"trusted key "no-separator""#));
}

#[test]
fn concurrency_never_drops_below_one() {
    let subs = substituters(&[], true).with_concurrency(0);
    assert_eq!(subs.concurrency, 1);
}

#[tokio::test]
async fn a_signature_under_a_trusted_name_but_another_secret_is_missing() {
    let dir = tempfile::tempdir().expect("tempdir");
    let path = store_path(1, "hello-2.12");
    let url = file_cache(dir.path(), &[(&path, narinfo(&path, Some(FORGED)))]);
    let subs = substituters(&[url], true);
    assert_eq!(ask(&subs, &path).await, Availability::Missing);
}

#[tokio::test]
async fn an_unreadable_narinfo_file_is_unknown() {
    let dir = tempfile::tempdir().expect("tempdir");
    let path = store_path(1, "hello-2.12");
    std::fs::create_dir(dir.path().join(narinfo_name(&path))).expect("mkdir");
    let url = file_cache(dir.path(), &[]);
    let subs = substituters(&[url], true);
    assert!(matches!(
        ask(&subs, &path).await,
        Availability::Unknown { .. }
    ));
}

fn serving(path: &StorePath) -> HashMap<String, (u16, String)> {
    HashMap::from([(
        format!("/{}", narinfo_name(path)),
        (200, narinfo(path, Some(SECRET))),
    )])
}

#[tokio::test]
async fn after_the_first_misses_the_rest_are_asked_at_once() {
    let path = store_path(1, "hello-2.12");
    let gate = Arc::new(Barrier::new(2));
    let first = http_cache(HashMap::new()).await;
    let second = http_cache_gated(HashMap::new(), Some(gate.clone()), vec![]).await;
    let third = http_cache_gated(serving(&path), Some(gate), vec![]).await;
    let subs = substituters(&[first, second, third.clone()], true);
    let answer = tokio::time::timeout(Duration::from_secs(5), ask(&subs, &path))
        .await
        .expect("the second and third substituters are asked together");
    assert_eq!(answer, Availability::Served { substituter: third });
}

#[tokio::test]
async fn the_earliest_configured_hit_wins_even_when_a_later_one_answers_first() {
    let path = store_path(1, "hello-2.12");
    let first = http_cache(HashMap::new()).await;
    let slow = http_cache_gated(serving(&path), None, vec![Duration::from_millis(200)]).await;
    let fast = http_cache(serving(&path)).await;
    let subs = substituters(&[first, slow.clone(), fast], true);
    assert_eq!(
        ask(&subs, &path).await,
        Availability::Served { substituter: slow }
    );
}

/// The retry answers in 300 ms, past the first patience of 100 ms but within
/// the retry's eightfold one.
#[tokio::test]
async fn a_stalled_request_is_retried_once_with_more_patience() {
    let path = store_path(1, "hello-2.12");
    let url = http_cache_gated(
        serving(&path),
        None,
        vec![Duration::from_secs(30), Duration::from_millis(300)],
    )
    .await;
    let subs =
        substituters(std::slice::from_ref(&url), true).with_patience(Duration::from_millis(100));
    let answer = tokio::time::timeout(Duration::from_secs(5), ask(&subs, &path))
        .await
        .expect("the retry answers");
    assert_eq!(answer, Availability::Served { substituter: url });
}

#[tokio::test]
async fn two_stalls_disable_the_substituter_and_say_so() {
    let path = store_path(1, "hello-2.12");
    let url = http_cache_gated(serving(&path), None, vec![Duration::from_secs(30)]).await;
    let subs =
        substituters(std::slice::from_ref(&url), true).with_patience(Duration::from_millis(50));
    let answer = tokio::time::timeout(Duration::from_secs(5), ask(&subs, &path))
        .await
        .expect("gives up in time");
    assert_eq!(answer, Availability::Missing);
    assert_eq!(
        subs.warnings(),
        vec![format!(
            "{url}: unreachable, not asked again: timed out twice"
        )]
    );
    let again = tokio::time::timeout(Duration::from_millis(40), ask(&subs, &path))
        .await
        .expect("a disabled substituter is not waited on");
    assert_eq!(again, Availability::Missing);
}

/// A URL nothing listens on: a port that was bound, then released.
fn refused() -> (String, std::net::SocketAddr) {
    let listener = std::net::TcpListener::bind("127.0.0.1:0").expect("bind");
    let addr = listener.local_addr().expect("local addr");
    (format!("http://{addr}"), addr)
}

#[tokio::test]
async fn a_refused_connection_disables_the_substituter_after_one_failure() {
    let (url, addr) = refused();
    let subs = substituters(std::slice::from_ref(&url), true);
    assert_eq!(
        ask(&subs, &store_path(1, "hello-2.12")).await,
        Availability::Missing
    );
    let warnings = subs.warnings();
    assert_eq!(warnings.len(), 1, "{warnings:?}");
    assert!(
        warnings
            .iter()
            .all(|warning| warning.starts_with(&format!("{url}: unreachable, not asked again: "))),
        "{warnings:?}"
    );
    let revived = tokio::net::TcpListener::bind(addr).await.expect("rebind");
    assert_eq!(
        ask(&subs, &store_path(2, "hello-2.12")).await,
        Availability::Missing
    );
    assert!(
        tokio::time::timeout(Duration::from_millis(100), revived.accept())
            .await
            .is_err(),
        "the second path was sent to the disabled substituter"
    );
    assert_eq!(subs.warnings(), warnings);
}

#[tokio::test]
async fn an_unreachable_substituter_does_not_hide_a_later_miss_or_hit() {
    let dir = tempfile::tempdir().expect("tempdir");
    let path = store_path(1, "hello-2.12");
    let cache = file_cache(dir.path(), &[(&path, narinfo(&path, Some(SECRET)))]);
    let subs = substituters(&[refused().0, cache.clone()], true);
    assert_eq!(
        ask(&subs, &store_path(2, "absent-1.0")).await,
        Availability::Missing
    );
    assert_eq!(
        ask(&subs, &path).await,
        Availability::Served { substituter: cache }
    );
}

/// `.invalid` never resolves (RFC 6761). With or without a resolver the
/// request fails before any answer, which disables the substituter.
#[tokio::test]
async fn a_name_that_does_not_resolve_disables_the_substituter() {
    let url = "http://nwb-no-such-host.invalid".to_owned();
    let subs =
        substituters(std::slice::from_ref(&url), true).with_patience(Duration::from_millis(500));
    assert_eq!(
        ask(&subs, &store_path(1, "hello-2.12")).await,
        Availability::Missing
    );
    let warnings = subs.warnings();
    assert_eq!(warnings.len(), 1, "{warnings:?}");
    assert!(warnings.iter().all(|warning| warning.starts_with(&url)));
}
