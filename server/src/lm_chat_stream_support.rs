use axum::{
    body::{Body, Bytes},
    http::{header, HeaderValue},
    response::Response,
};
use futures::StreamExt;
use serde_json::{json, Value};
use std::convert::Infallible;
use tokio::sync::mpsc;
use tokio_stream::wrappers::ReceiverStream;

pub(crate) fn stream_response(rx: mpsc::Receiver<String>) -> Response {
    let stream = ReceiverStream::new(rx).map(|chunk| Ok::<Bytes, Infallible>(Bytes::from(chunk)));
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
