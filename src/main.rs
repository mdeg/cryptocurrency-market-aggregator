extern crate url;
extern crate ws;
extern crate serde;
#[macro_use] extern crate serde_derive;
extern crate serde_json;
extern crate env_logger;

use std::thread;
use std::net::SocketAddr;
use std::str::FromStr;

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

fn main() {
    env_logger::init();

    let server = ws::WebSocket::new(move |out: ws::Sender| {
        // TODO: debug info
        println!("Client has connected to the server");

        out.send(serde_json::to_string(&Broadcast::Connected {}).unwrap()).unwrap();

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

    thread::spawn(move|| connect_btcmarkets_proxy(tx.clone(), currency, instrument));
//    thread::spawn(move|| connect_poloniex(tx.clone(), currency, instrument));

    loop {
        //TODO: send heartbeat
    }
}

#[derive(Debug, Serialize)]
enum Broadcast {
    Heartbeat {
        tick: bool
    },
    Connected {}
}

#[derive(Debug, Serialize)]
enum PoloniexRequest {
    JoinQueue {
        command: String,
        channel: String
    }
}

fn connect_poloniex(tx: ws::Sender, currency: &str, instrument: &str) {
    let connect_addr = "wss://api2.poloniex.com";

    ws::connect(connect_addr, |out| {
        println!("Successfully connected to Poloniex");

//        let join_pair = format!(r#"{{"command": "subscribe", "channel": "{}_{}"}}"#,
//            "BTC", "XMR"
//        );
        let request = PoloniexRequest::JoinQueue {
            command: "subscribe".to_string(),
            channel: format!("{}_{}", currency, instrument)
        };
        match out.send(serde_json::to_string(&request).unwrap()) {
            Ok(_) => println!("Joined Poloniex currency pair queue"),
            Err(e) => println!("Could not join Poloniex currency pair queue: {}", e)
        }

//        let join_hb = r#"{"command": "subscribe", "channel": 1010}"#;
        let request = PoloniexRequest::JoinQueue {
            command: "subscribe".to_string(),
            channel: "1010".to_string()
        };
        match out.send(serde_json::to_string(&request).unwrap()) {
            Ok(_) => println!("Joined Poloniex heartbeat queue"),
            Err(e) => println!("Could not join Poloniex heartbeat queue: {}", e)
        }

        let tmp = tx.clone();
        move |msg: ws::Message| {

            println!("Raw message: {}", msg);

            if msg.is_text() {
                match msg.into_text() {
                    Ok(txt) => {
                        match serde_json::from_str::<PoloniexResponse>(&txt) {
                            Ok(response) => {
                                let serialized =
                                    serde_json::to_string(&Broadcast::Heartbeat { tick: true }).unwrap();
                                if let Err(e) = tmp.send(serialized) {
                                    println!("Could not send passthrough message: {}", e);
                                }
                            },
                            Err(e) => panic!("Could not deserialize Poloniex message {}: {}", txt, e)
                        };
                    },
                    Err(e) => println!("Could not convert text-based Poloniex message into text: {}", e)
                };
            } else {
                println!("Binary message");
            }

            Ok(())
        }
    });
}

#[derive(Debug, Deserialize)]
#[serde(tag = "type")]
enum PoloniexResponse {
    #[serde(rename = "orderBookModify")]
    OrderBookModify {
        rate: f64,
        #[serde(rename = "type")]
        order_type: String,
        amount: f64
    },
    #[serde(rename = "orderBookRemove")]
    OrderBookRemove {
        rate: f64,
        #[serde(rename = "type")]
        order_type: String
    },
    #[serde(rename = "newTrade")]
    Trade {
        #[serde(rename = "tradeId")]
        trade_id: String,
        rate: f64,
        amount: f64,
        date: String,
        total: f64,
        #[serde(rename = "type")]
        trade_type: String
    }
}

fn connect_btcmarkets_proxy(tx: ws::Sender, currency: &str, instrument: &str) {
//    let connect_addr = "wss://socket.btcmarkets.net";
    let connect_addr = "ws://localhost:10001";

    ws::connect(connect_addr, |out| {
        println!("Successfully connected to BTCMarkets");

        let orderbook_join = format!(r#"{{"channelName": "Orderbook_{}{}", "eventName": "OrderBookChange"}}"#,
            instrument, currency);
        let trade_join = format!(r#"{{"channelName": "TRADE_{}{}", "eventName": "MarketTrade"}}"#,
            instrument, currency);

        match out.send(orderbook_join) {
            Ok(_) => println!("Joined orderbook event queue"),
            Err(e) => println!("Could not join orderbook event queue: {}", e)
        }
        match out.send(trade_join) {
            Ok(_) => println!("Joined trade event queue"),
            Err(e) => println!("Could not join trade event queue: {}", e)
        }

        // TODO: be less dumb with this temporary variable
        let tmp = tx.clone();
        move |msg: ws::Message| {

            println!("Raw message: {}", msg);

            if msg.is_text() {
                match msg.into_text() {
                    Ok(txt) => {
                        match serde_json::from_str::<BtcMarketsResponse>(&txt) {
                            Ok(response) => {
                                //TODO: map it
                                let serialized =
                                    serde_json::to_string(&Broadcast::Heartbeat { tick: true }).unwrap();
                                if let Err(e) = tmp.send(serialized) {
                                     println!("Could not send broadcast: {}", e);
                                }
                            },
                            Err(e) => println!("Could not deserialize BTCMarkets message {}: {}", txt, e)
                        };
                    },
                    Err(e) => println!("Could not convert text-based BTCMarkets message into text: {}", e)
                };
            } else {
                println!("Binary message");
            }

            Ok(())
        }
    }).unwrap();

}



#[derive(Debug, Deserialize)]
#[serde(untagged)]
enum BtcMarketsResponse {
    OrderbookChange {
        currency: String,
        instrument: String,
        timestamp: i64,
        #[serde(rename = "marketId")]
        market_id: i64,
        #[serde(rename = "snapshotId")]
        snapshot_id: i64,
        bids: Vec<(i64, i64, i64)>,
        asks: Vec<(i64, i64, i64)>
    },
    Trade {
        id: i64,
        timestamp: i64,
        #[serde(rename = "marketId")]
        market_id: i64,
        agency: String,
        instrument: String,
        currency: String,
        trades: Vec<(i64, i64, i64, i64)>
    },
    Status {
        status: String
    },
}