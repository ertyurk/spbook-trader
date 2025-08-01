// Event bus for internal message passing

use quant_models::MatchEvent;
use tokio::sync::mpsc;

pub struct EventBus {
    sender: mpsc::UnboundedSender<MatchEvent>,
    receiver: mpsc::UnboundedReceiver<MatchEvent>,
}

impl EventBus {
    pub fn new() -> Self {
        let (sender, receiver) = mpsc::unbounded_channel();
        Self { sender, receiver }
    }
}