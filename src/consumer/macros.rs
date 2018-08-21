macro_rules! generic_open {
    ($exch:path) => {
        fn on_open(&mut self, _: ws::Handshake) -> ws::Result<()> {
            info!("Connected to {}", $exch);

            let requests = Self::get_requests(&self.pairs);

            while let Err(e) = self.inner.send_upstream(&requests) {
                // This is a worry - might indicate a busted exchange
                // All queue messages need to reach the endpoint to guarantee downstream data validity

                error!("Could not send all channel join requests to {}: {}", $exch, e);

                // TODO: check what happens on multiple subscriptions

                info!("Retrying connection to {} in 10 seconds...", $exch);
                ::std::thread::sleep(::std::time::Duration::from_secs(10));
            }

            let open = Broadcast::ExchangeConnectionOpened {
                exchange: $exch,
                ts: consumer::timestamp()
            };

            if let Err(e) = self.inner.broadcast(BroadcastType::One(open)) {
                // May occur on launch when exchange connection is fine but server has not started up yet
                warn!("Could not broadcast {} open message: {}", $exch, e);
            }

            Ok(())
        }
    }
}

macro_rules! generic_on_message {
    ($resp:ty) => {
        fn on_message(&mut self, msg: ws::Message) -> ws::Result<()> {
            match msg.into_text() {
                Ok(txt) => {
                    match ::serde_json::from_str::<$resp>(&txt) {
                        Ok(response) => {
                            let broadcast = self.handle_response(response);

                            if let Err(e) = self.inner.broadcast(broadcast) {
                                error!("Could not broadcast message: {}", e);
                            }
                        },
                        Err(e) => error!("Could not deserialize message: {}", e)
                    }
                },
                Err(e) => error!("Could not convert message to text: {}", e)
            };

            Ok(())
        }
    }
}

macro_rules! generic_on_close {
    ($exch:path) => {
        fn on_close(&mut self, _code: ws::CloseCode, reason: &str) {
            info!("Connection to {} has been lost: {}", $exch, reason);

            let closed = Broadcast::ExchangeConnectionClosed {
                exchange: $exch,
                ts: consumer::timestamp()
            };

            if let Err(e) = self.inner.broadcast(BroadcastType::One(closed)) {
                // This might occur if the broadcast connection disappears completely
                // For example, loss of network connectivity
                // Clients should rely on the missing heartbeat to determine that data is invalid
                warn!("Could not broadcast exchange closed message: {}", e);
            }
        }
    }
}
