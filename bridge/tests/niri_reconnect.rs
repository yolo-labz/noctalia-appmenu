//! Integration tests for the niri-IPC connect / ack path (FR-002 of
//! spec 005-bridge-completion).
//!
//! The fixture stands up a tokio `UnixListener` at a tempfile path,
//! invokes `niri::run_once_at` against that socket, and scripts the
//! server side to produce the ack-path scenarios we want to cover:
//!
//!   1. Happy path — server writes `Ok(Response::Handled)` then EOFs.
//!      `run_once_at` must return `Err` (the documented "session
//!      ended" path) without a parse error in the chain — proving
//!      the canonical ack is parsed.
//!   2. Malformed ack — server writes invalid JSON. `run_once_at`
//!      must return `Err` with a parse-error context, NOT silently
//!      enter the backoff loop.
//!   3. Rejected subscription — server writes `Err("denied")`.
//!      `run_once_at` must surface the rejection text.
//!
//! Each test runs in its own tempdir so the listener paths don't
//! collide under `cargo test`'s parallel test runner.

use noctalia_appmenu_bridge::niri::run_once_at;
use tempfile::TempDir;
use tokio::io::{AsyncBufReadExt, AsyncWriteExt, BufReader};
use tokio::net::UnixListener;
use tokio::sync::watch;

/// Spawn a fake niri server on a `UnixListener` bound to a temp path.
/// The closure runs *inside* the accept loop with the accepted
/// (read-half, write-half) stream — the test scripts the bytes to
/// send back.
async fn fake_niri<F, Fut>(handler: F) -> (TempDir, std::path::PathBuf, tokio::task::JoinHandle<()>)
where
    F: FnOnce(
            tokio::io::BufReader<tokio::net::unix::OwnedReadHalf>,
            tokio::net::unix::OwnedWriteHalf,
        ) -> Fut
        + Send
        + 'static,
    Fut: std::future::Future<Output = ()> + Send,
{
    let dir = TempDir::new().expect("tempdir");
    let path = dir.path().join("niri.sock");
    let listener = UnixListener::bind(&path).expect("bind unix listener");
    let path_for_task = path.clone();
    let handle = tokio::spawn(async move {
        let (stream, _) = listener.accept().await.expect("accept");
        let (rd, wr) = stream.into_split();
        handler(BufReader::new(rd), wr).await;
        // Keep `path_for_task` in scope so the bind path survives until
        // the client connects. The bind itself ensures the file exists
        // on disk for `UnixStream::connect`.
        drop(path_for_task);
    });
    (dir, path, handle)
}

/// Read one newline-delimited line from the client's request half
/// (it will be the EventStream request) and discard it.
async fn drain_request(rd: &mut tokio::io::BufReader<tokio::net::unix::OwnedReadHalf>) -> String {
    let mut req = String::new();
    rd.read_line(&mut req).await.expect("read request");
    req
}

#[tokio::test]
async fn ack_path_happy_then_eof_returns_session_ended_err() {
    let (_dir, path, _server) = fake_niri(|mut rd, mut wr| async move {
        let req = drain_request(&mut rd).await;
        assert!(
            req.contains("EventStream"),
            "expected client to send EventStream request, got {req:?}"
        );
        // Canonical ack — Ok(Response::Handled) serialises as
        // `{"Ok":"Handled"}`. niri-ipc 26.4 wire format; if niri
        // changes this in a future minor, run_once parsing will
        // fail loudly here.
        wr.write_all(b"{\"Ok\":\"Handled\"}\n")
            .await
            .expect("write ack");
        // Now close the write half — drive_events will EOF and
        // run_once_at returns Err("niri event-stream closed").
        wr.shutdown().await.expect("shutdown");
    })
    .await;

    let (tx, _rx) = watch::channel(None);
    let err = run_once_at(&path, &tx)
        .await
        .expect_err("session must err on EOF");
    let s = format!("{err:#}");
    assert!(
        s.contains("niri event-stream closed"),
        "EOF after a parsed ack should surface as the 'session ended' err, got {s:?}"
    );
}

#[tokio::test]
async fn ack_path_malformed_response_returns_parse_err() {
    let (_dir, path, _server) = fake_niri(|mut rd, mut wr| async move {
        let _ = drain_request(&mut rd).await;
        // Malformed ack — looks like JSON but is not a valid Reply.
        // run_once_at must return Err with the parse-context chain,
        // not silently loop in backoff.
        wr.write_all(b"{\"this\":\"is not a Reply\"}\n")
            .await
            .expect("write garbage");
        wr.shutdown().await.expect("shutdown");
    })
    .await;

    let (tx, _rx) = watch::channel(None);
    let err = run_once_at(&path, &tx)
        .await
        .expect_err("malformed ack must err");
    let s = format!("{err:#}");
    assert!(
        s.contains("parsing EventStream ack"),
        "malformed ack must surface as 'parsing EventStream ack' err, got {s:?}"
    );
}

#[tokio::test]
async fn ack_path_rejected_subscription_returns_typed_err() {
    let (_dir, path, _server) = fake_niri(|mut rd, mut wr| async move {
        let _ = drain_request(&mut rd).await;
        // niri rejects the subscription. Reply::Err carries a string.
        wr.write_all(b"{\"Err\":\"subscription denied\"}\n")
            .await
            .expect("write rejection");
        wr.shutdown().await.expect("shutdown");
    })
    .await;

    let (tx, _rx) = watch::channel(None);
    let err = run_once_at(&path, &tx)
        .await
        .expect_err("rejection must err");
    let s = format!("{err:#}");
    assert!(
        s.contains("niri rejected EventStream subscription"),
        "rejection must surface as 'niri rejected ...' err, got {s:?}"
    );
    assert!(
        s.contains("subscription denied"),
        "rejection should preserve the upstream reason, got {s:?}"
    );
}

#[tokio::test]
async fn ack_path_socket_closed_before_ack_returns_err() {
    let (_dir, path, _server) = fake_niri(|mut rd, wr| async move {
        // Accept the request, then close immediately — no ack.
        let _ = drain_request(&mut rd).await;
        // Drop wr (and rd) to close both halves of the socket
        // before sending the ack line.
        drop(wr);
        drop(rd);
    })
    .await;

    let (tx, _rx) = watch::channel(None);
    let err = run_once_at(&path, &tx)
        .await
        .expect_err("ack-less close must err");
    let s = format!("{err:#}");
    assert!(
        s.contains("niri closed socket before EventStream ack"),
        "premature close must err with 'closed socket before ... ack', got {s:?}"
    );
}
