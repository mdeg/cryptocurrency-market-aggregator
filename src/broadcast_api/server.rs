use super::Broadcast;

use std::thread;
use std::str::FromStr;
use ws;

pub struct Server {
    // Channel that funnels broadcasts to the WebSocket
    tx: ws::Sender,
    // Store the heartbeat to prevent reserializing it
    hb: String
}

impl Server {
    pub fn run() -> Self {

        let server = ws::Builder::new().with_settings({
            let mut settings = ws::Settings::default();
            settings.tcp_nodelay = true;
            settings.panic_on_internal = false;
            settings.panic_on_new_connection = true;
            settings
        }).build(move |out: ws::Sender| {
            info!("Client has connected to the server");

            // Broadcast a connected message to clients when they hook into the broadcast API
            let connected = Broadcast::Connected { multiplier: ::MULTIPLIER };
            let connected_serialized = ::serde_json::to_string(&connected)
                .expect("Could not serialize connection message - this should never happen!");

            out.send(connected_serialized)
                .unwrap_or_else(|e| error!("Could not send connected message to client: {}", e));

            move |msg| {
                debug!("Got message from client: {}", msg);

                // If handling messages from clients is needed, this should be done here

                Ok(())
            }
        }).expect("Could not create WebSocket broadcast server!");

        let tx = server.broadcaster().clone();

        // Kick off a thread with our running server inside it
        thread::spawn(move || {
            let addr = dotenv!("SERVER_ADDR");
            let socket_addr = ::std::net::SocketAddr::from_str(dotenv!("SERVER_ADDR"))
                .expect(&format!("Could not establish server on {} - recheck the environment file values", &addr));
            match server.listen(&socket_addr) {
                Ok(_) => info!("Broadcast listen socket closed gracefully"),
                Err(e) => error!("Broadcast listen socket ended in an error: {}", e)
            }
        });

        let hb = ::serde_json::to_string(&Broadcast::Heartbeat {})
            .expect("Could not serialize heartbeat - this should never happen!");

        Self { tx, hb }
    }

    pub fn heartbeat(&self) {
        self.tx.send((&self.hb).as_str()).unwrap_or_else(|e| error!("Could not send heartbeat: {}", e));
    }

    pub fn tx(&self) -> ws::Sender {
        self.tx.clone()
    }
}