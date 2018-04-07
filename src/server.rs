use common::Broadcast;
use std::thread;
use std::str::FromStr;

pub struct Server {
    tx: ::ws::Sender
}

impl Server {
    pub fn run() -> Self {
        let server = ::ws::WebSocket::new(move |out: ::ws::Sender| {
            info!("Client has connected to the server");

            let connected = Broadcast::Connected { multiplier: ::MULTIPLIER };
            out.send(::serde_json::to_string(&connected).unwrap())
                .unwrap_or_else(|e| error!("Could not send connected message to client: {}", e));

            move |msg| {
                debug!("Got message from client: {}", msg);
                Ok(())
            }
        }).unwrap();

        let tx = server.broadcaster().clone();

        thread::spawn(move || {
            let addr = ::std::net::SocketAddr::from_str(dotenv!("SERVER_ADDR")).unwrap();
            server.listen(addr).unwrap();
        });

        Self {
            tx
        }
    }

    pub fn heartbeat(&self) {
        // TODO: prevent serializing this on every loop
        let hb = ::serde_json::to_string(&Broadcast::Heartbeat {}).unwrap();
        self.tx.send(hb).unwrap_or_else(|e| error!("Could not send heartbeat: {}", e));
    }

    pub fn tx(&self) -> ::ws::Sender {
        self.tx.clone()
    }
}