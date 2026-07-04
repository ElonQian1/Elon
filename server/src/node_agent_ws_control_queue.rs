use tokio::sync::mpsc;
use tokio_tungstenite::tungstenite::Message;

pub(crate) async fn recv(
    control_rx: &mut mpsc::UnboundedReceiver<Message>,
    out_rx: &mut mpsc::UnboundedReceiver<Message>,
) -> Option<Message> {
    tokio::select! {
        biased;
        msg = control_rx.recv() => match msg {
            Some(msg) => Some(msg),
            None => out_rx.recv().await,
        },
        msg = out_rx.recv() => msg,
    }
}

pub(crate) fn send_pong(tx: &mpsc::UnboundedSender<Message>, payload: Vec<u8>) {
    let _ = tx.send(Message::Pong(payload));
}
