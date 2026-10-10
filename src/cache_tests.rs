use std::collections::HashMap;
use std::path::Path;

use nix_compat::narinfo::{NarInfo, parse_keypair};
use nix_compat::nixbase32;
use nix_compat::store_path::StorePath;
use tokio::io::{AsyncReadExt, AsyncWriteExt};

use super::{Availability, Substituters};

const SECRET: &str = "test-cache-1:2EJ0WJNsrKIJRZSuip51j78aJfN8nPVFj/geAmnSeaa1qx20A+Oh9p6Lwvk6VlxBeipwM3pm5VprKHOI+OzCTA==";
const PUBLIC: &str = "test-cache-1:tasdtAPjofaei8L5OlZcQXoqcDN6ZuVaayhziPjswkw=";
const ROGUE: &str = "rogue-1:WSWrsHVFjRnBaZytl7Cdh0eHb6zbSz9y24G6xtmQbVWvi0KYP2v7rj4+gTTXRbYWWsryCjAF3oGDOq3FTlJeEA==";
/// Claims the trusted key's name, signs with another secret.
const FORGED: &str = "test-cache-1:lt2k779oTCU/mF2pbGitQK2tcmjqAMhi9KRBA5s7Dkg54t8HZgNEyhYbmGfTmyfSgLV6C6yQZrm6zMjxsTGkUQ==";

fn store_path(seed: u8, name: &str) -> StorePath {
    StorePath::from_name_and_digest_fixed(name, [seed; 20]).expect("valid store path")
}

/// A narinfo for `path`, signed with `secret` when one is given.
fn narinfo(path: &StorePath, secret: Option<&str>) -> String {
    let text = format!(
        "StorePath: {}\nURL: nar/x.nar.xz\nCompression: xz\nNarHash: sha256:{}\nNarSize: 42\nReferences: \n",
        path.to_absolute_path(),
        nixbase32::encode(&[7; 32]),
    );
    let mut info = NarInfo::parse(&text).expect("valid narinfo");
    let Some(secret) = secret else {
        return info.to_string();
    };
    let (signer, _) = parse_keypair(secret).expect("valid keypair");
    info.add_signature(&signer);
    info.to_string()
}

fn narinfo_name(path: &StorePath) -> String {
    format!("{}.narinfo", nixbase32::encode(path.digest()))
}

fn file_cache(dir: &Path, entries: &[(&StorePath, String)]) -> String {
    for (path, body) in entries {
        std::fs::write(dir.join(narinfo_name(path)), body).expect("write narinfo");
    }
    format!("file://{}", dir.display())
}

fn substituters(urls: &[String], require_sigs: bool) -> Substituters {
    Substituters::new(urls, &[PUBLIC.to_owned()], require_sigs).expect("valid substituters")
}

async fn ask(subs: &Substituters, path: &StorePath) -> Availability {
    let answers = subs.query(std::slice::from_ref(path)).await;
    answers.get(path).cloned().expect("an answer per path")
}

/// Serves `routes` (URL path to status and body) over HTTP/1.1, one request
/// per connection, and returns the base URL.
async fn http_cache(routes: HashMap<String, (u16, String)>) -> String {
    let listener = tokio::net::TcpListener::bind("127.0.0.1:0")
        .await
        .expect("bind");
    let addr = listener.local_addr().expect("local addr");
    tokio::spawn(async move {
        while let Ok((mut socket, _)) = listener.accept().await {
            let mut buf = vec![0_u8; 4096];
            let read = socket.read(&mut buf).await.unwrap_or(0);
            let request = String::from_utf8_lossy(buf.get(..read).unwrap_or_default()).into_owned();
            let target = request.split_whitespace().nth(1).unwrap_or("/").to_owned();
            let (status, body) = routes.get(&target).cloned().unwrap_or((404, String::new()));
            let response = format!(
                "HTTP/1.1 {status} X\r\nContent-Length: {}\r\nConnection: close\r\n\r\n{body}",
                body.len()
            );
            let _ = socket.write_all(response.as_bytes()).await;
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
    let routes = HashMap::from([(format!("/{}", narinfo_name(&path)), (503, String::new()))]);
    let url = http_cache(routes).await;
    let subs = substituters(std::slice::from_ref(&url), true);
    assert_eq!(
        ask(&subs, &path).await,
        Availability::Unknown {
            reason: format!("{url}: HTTP 503 Service Unavailable")
        }
    );
}

#[tokio::test]
async fn a_later_hit_beats_an_earlier_failure() {
    let full = tempfile::tempdir().expect("tempdir");
    let path = store_path(1, "hello-2.12");
    let served = file_cache(full.path(), &[(&path, narinfo(&path, Some(SECRET)))]);
    let subs = substituters(&["ssh://builder".to_owned(), served.clone()], true);
    assert_eq!(
        ask(&subs, &path).await,
        Availability::Served {
            substituter: served
        }
    );
}

#[tokio::test]
async fn an_unsupported_scheme_is_unknown() {
    let subs = substituters(&["ssh://builder".to_owned()], true);
    assert_eq!(
        ask(&subs, &store_path(1, "hello-2.12")).await,
        Availability::Unknown {
            reason: "ssh://builder: unsupported substituter scheme".to_owned()
        }
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
        format!("{:?}", subs.list.get(1).map(|(_, location)| location)),
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
