use axum::{
    body::{Body, Bytes},
    http::{header, HeaderValue},
    response::Response,
};
use futures::stream;
use serde_json::{json, Value};
use std::convert::Infallible;
use std::time::Duration;
use tokio::sync::mpsc;

pub(crate) fn stream_response(rx: mpsc::Receiver<String>) -> Response {
    // Keep long-running AI/tool orchestration streams alive through proxies and
    // browser network stacks that close an otherwise quiet HTTP connection.
    // SSE comments are ignored by the client but reset idle timeouts.
    let heartbeat = tokio::time::interval_at(
        tokio::time::Instant::now() + Duration::from_secs(10),
        Duration::from_secs(10),
    );
    let stream = stream::unfold((rx, heartbeat), |(mut rx, mut heartbeat)| async move {
        tokio::select! {
            chunk = rx.recv() => match chunk {
                Some(chunk) => Some((
                    Ok::<Bytes, Infallible>(Bytes::from(chunk)),
                    (rx, heartbeat),
                )),
                None => None,
            },
            _ = heartbeat.tick() => Some((
                Ok::<Bytes, Infallible>(Bytes::from_static(b": keep-alive\n\n")),
                (rx, heartbeat),
            )),
        }
    });
    let mut response = Response::new(Body::from_stream(stream));
    response.headers_mut().insert(
        header::CONTENT_TYPE,
        HeaderValue::from_static("text/event-stream; charset=utf-8"),
    );
    response.headers_mut().insert(
        header::CACHE_CONTROL,
        HeaderValue::from_static("no-cache, no-transform"),
    );
    response
        .headers_mut()
        .insert(header::CONNECTION, HeaderValue::from_static("keep-alive"));
    response.headers_mut().insert(
        header::HeaderName::from_static("x-accel-buffering"),
        HeaderValue::from_static("no"),
    );
    response
}

pub(crate) async fn send_stream_event(tx: &mpsc::Sender<String>, payload: Value) -> bool {
    tx.send(format!("data: {}\n\n", payload)).await.is_ok()
}

pub(crate) async fn send_stream_error(tx: &mpsc::Sender<String>, message: impl Into<String>) {
    let _ = send_stream_event(
        tx,
        json!({
            "type": "error",
            "message": message.into(),
        }),
    )
    .await;
}
