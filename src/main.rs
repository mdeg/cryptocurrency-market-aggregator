extern crate url;
extern crate ws;
extern crate serde;
#[macro_use] extern crate serde_derive;
extern crate serde_json;
extern crate env_logger;

mod poloniex;
mod btcmarkets;

use std::thread;
use std::net::SocketAddr;
use std::str::FromStr;
use std::time::Duration;

// 1. Get websocket server working
// TODO POC
// 2. Work out what an orderbook IS
// 3. Reverse Poloniex's new API
// 4. Work out message format

// TODO LATER
// Sort out project structure
// Environment variables
// Logger setup

// TODO EVENTUALLY
// Start script
// Bindings for JS proxy

const MULTIPLIER: i32 = 100000000;

fn main() {
    env_logger::init();

    let server = ws::WebSocket::new(move |out: ws::Sender| {
        // TODO: debug info
        println!("Client has connected to the server");

        let connected = Broadcast::Connected { multiplier: MULTIPLIER };
        out.send(serde_json::to_string(&connected).unwrap()).unwrap();

        move |msg| {
            println!("Got message from client: {}", msg);
            Ok(())
        }
    }).unwrap();

    let tx = server.broadcaster();
    thread::spawn(move || {
        let addr = SocketAddr::from_str("127.0.0.1:60400").unwrap();
        server.listen(addr).unwrap();
    });

    let currency = "AUD";
    let instrument = "BTC";

    let mut tx_in = tx.clone();
    thread::spawn(move || btcmarkets::connect(tx_in, currency, instrument, true));
    tx_in = tx.clone();
//    thread::spawn(move|| poloniex::connect(tx_in, currency, instrument));

    let hb = Broadcast::Heartbeat {};
    loop {
        thread::sleep(Duration::from_secs(1));
        if let Err(e) = tx.send(serde_json::to_string(&hb).unwrap()) {
            println!("Could not send heartbeat: {}", e);
        }
    }
}

#[derive(Debug, Serialize)]
enum Broadcast {
    #[serde(rename = "hb")]
    Heartbeat {},
    #[serde(rename = "orderbookUpdate")]
    OrderbookUpdate {
        seq_num: i32,
        source: Exchange,
        pair: (String, String),
        bids: Vec<(i64, i64)>,
        asks: Vec<(i64, i64)>
    },
    #[serde(rename = "trade")]
    Trade {
        seq_num: i32,
        source: Exchange
    },
    #[serde(rename = "connected")]
    Connected {
        multiplier: i32
    }
}

#[derive(Debug, Serialize)]
enum Exchange {
    #[serde(rename = "btcmarkets")]
    BtcMarkets,
    #[serde(rename = "poloniex")]
    Poloniex
}
