use ws;
use broadcast_api::BroadcastType;
use super::error::*;

pub struct HandlerCore {
    // Sender to broadcast to consumers connected to this program
    broadcast_tx: ws::Sender,
    // Sender to message inbound streams from the exchange
    exchange_tx: ws::Sender
}

impl HandlerCore {

    pub fn new(broadcast_tx: ws::Sender, exchange_tx: ws::Sender) -> Self {
        Self { broadcast_tx, exchange_tx }
    }

    // Send messages upstream to the API we are consuming from
    pub fn send_upstream(&mut self, msgs: &[String]) -> Result<()> {
        let failures: Vec<Error> = msgs.into_iter().map(|msg| {
                self.exchange_tx.send(msg.clone())
                    .chain_err(|| ErrorKind::BroadcastError)
            })
            .filter(|result| result.is_err())
            .map(|fail| fail.unwrap_err())
            .collect();

        if failures.len() == 0 {
            Ok(())
        } else {
            bail!(ErrorKind::MultipleBroadcastError(failures))
        }
    }

    // Broadcast messages downstream to the consumers listening to our broadcast
    pub fn broadcast(&mut self, broadcast: BroadcastType) -> Result<()> {
        match broadcast {
            BroadcastType::None => {
                trace!("Discarding message - no broadcast required");

                Ok(())
            },
            BroadcastType::One(broadcast) => {
                trace!("Sending one broadcast");

                let serialized = ::serde_json::to_string(&broadcast)?;
                self.broadcast_tx.send(serialized)?;

                Ok(())
            },
            BroadcastType::Many(broadcasts) => {
                trace!("Sending {} broadcasts", broadcasts.len());

                let serialization: Vec<Result<String>> = broadcasts.into_iter()
                    .map(|broadcast| ::serde_json::to_string(&broadcast)
                        .chain_err(|| ErrorKind::BroadcastError))
                    .collect();

                let mut failures = vec!();

                for serialization_result in serialization {
                    match serialization_result {
                        Ok(msg) => {
                            if let Err(e) = self.broadcast_tx.send(msg).chain_err(|| ErrorKind::BroadcastError) {
                                failures.push(e);
                            }
                        },
                        Err(e) => failures.push(e)
                    }
                }

                if failures.len() == 0 {
                    Ok(())
                } else {
                    bail!(ErrorKind::MultipleBroadcastError(failures))
                }
            }
        }
    }
}