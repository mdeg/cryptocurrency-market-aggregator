pub mod handler;

mod error;

use super::domain::*;
use ws;
use std::{time, thread};

pub fn connect<T: ws::Factory + ConnectionFactory>(broadcast_tx: ws::Sender, pairs: Vec<CurrencyPair>) {
    thread::spawn(move || {
        loop {
            let factory = T::new(broadcast_tx.clone(), pairs.clone());

            match ws::Builder::new().build(factory) {
                Ok(mut ws) => {
                    match ws.connect(T::get_connect_addr()) {
                        Ok(_) => {
                            match ws.run() {
                                Ok(_) => info!("WebSocket connection closed gracefully"),
                                Err(e) => error!("WebSocket connection failed: {}", e)
                            }
                        },
                        Err(e) => error!("Could not queue WebSocket connection: {}", e)
                    }
                },
                Err(e) => error!("Could not construct WebSocket using factory: {}", e)
            }

            // We've lost connection to our WebSocket endpoint (or could not build it)
            // Consumers will have been notified of this event through the handler
            // Delay then attempt a reconnect
            thread::sleep(time::Duration::from_secs(10));
            info!("Attempting to reconnect...");
        }
    });
}

pub trait ConnectionFactory {
    fn new(broadcast_tx: ::ws::Sender, pairs: Vec<CurrencyPair>) -> Self;

    fn get_connect_addr() -> ::url::Url;
}

pub trait MarketHandler {
    fn get_requests(pairs: &[CurrencyPair]) -> Vec<String>;

    fn stringify_pair(pair: &CurrencyPair) -> String;
}

pub fn standardise_value(value: f64) -> i64 {
    (value * f64::from(::MULTIPLIER)).round() as i64
}

pub fn timestamp() -> i64 {
    let time = time::SystemTime::now()
        .duration_since(time::UNIX_EPOCH)
        .expect("Time went backwards");

    (time.as_secs() * 1000) as i64
}