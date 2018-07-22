use ws;
use broadcast_api::BroadcastType;

pub struct HandlerCore {
    // Sender to broadcast to consumers connected to this program
    broadcast_tx: ws::Sender,
    // Sender to message inbound streams from the exchange
    exchange_tx: ws::Sender
}

// TODO: work out how to handle errors thrown in the core here
impl HandlerCore {

    pub fn new(broadcast_tx: ws::Sender, exchange_tx: ws::Sender) -> Self {
        Self { broadcast_tx, exchange_tx }
    }

    pub fn send_upstream(&mut self, requests: Vec<String>) {
        requests.into_iter().for_each(|req| {
            self.exchange_tx.send(req)
                .unwrap_or_else(|e| error!("Failed to send request: {}", e));
        });
    }

    pub fn broadcast(&mut self, broadcast: BroadcastType) -> ws::Result<()> {
        match broadcast {
            BroadcastType::None => trace!("Discarding message - no broadcast required"),
            BroadcastType::One(broadcast) => {
                trace!("Sending one broadcast");

                self.broadcast_tx.send(::serde_json::to_string(&broadcast).unwrap())
                    .unwrap_or_else(|e| error!("Could not broadcast: {}", e))
            },
            BroadcastType::Many(broadcasts) => {
                trace!("Sending {} broadcasts", broadcasts.len());

                broadcasts.into_iter()
                    .map(|broadcast| ::serde_json::to_string(&broadcast).unwrap())
                    .for_each(|msg| self.broadcast_tx.send(msg)
                        .unwrap_or_else(|e| error!("Could not broadcast: {}", e)))
            }
        }

        Ok(())
    }
}