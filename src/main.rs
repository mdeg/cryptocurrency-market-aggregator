extern crate url;
extern crate ws;
extern crate serde;
#[macro_use] extern crate serde_derive;
extern crate serde_json;
extern crate env_logger;
extern crate simplelog;
#[macro_use] extern crate log;

// TODO: Poloniex support
//mod poloniex;
mod btcmarkets;
//mod bitfinex;
mod common;

use std::thread;
use std::str::FromStr;
use simplelog::*;
use common::{MarketRunner, Broadcast};

const MULTIPLIER: i32 = 100000000;

fn main() {
    let log_file = std::fs::File::create("./aggregator.log")
        .expect("Could not create log file");

    CombinedLogger::init(vec![
        TermLogger::new(LevelFilter::Debug, Config::default()).expect("Could not initialise terminal logger"),
        WriteLogger::new(LevelFilter::Debug, Config::default(), log_file),
    ]).expect("Could not initialise combined logger");

    let server = ws::WebSocket::new(move |out: ws::Sender| {
        // TODO: debug info for client
        info!("Client has connected to the server");

        let connected = Broadcast::Connected { multiplier: MULTIPLIER };
        out.send(serde_json::to_string(&connected).unwrap())
            .unwrap_or_else(|e| error!("Could not send connected message to client: {}", e));

        move |msg| {
            debug!("Got message from client: {}", msg);
            Ok(())
        }
    }).unwrap();

    let tx = server.broadcaster();
    thread::spawn(move || {
        let addr = ::std::net::SocketAddr::from_str("192.168.1.71:60400").unwrap();
        server.listen(addr).unwrap();
    });

    // TODO: take list from args
    let pair = (String::from("XRP"), String::from("BTC"));
    let pairs = vec!(pair);

    // TODO: better way of doing this than cloning
    let mut tx_in = tx.clone();
    let mut pairs_in = pairs.clone();
//    thread::spawn(move || bitfinex::BitfinexMarketRunner{}.connect(tx_in, pairs_in));
    tx_in = tx.clone();
    pairs_in = pairs.clone();
    thread::spawn(move || btcmarkets::BtcMarketsMarketRunner::new().connect(tx_in, pairs_in));

    // TODO: finish Poloniex support
//    tx_in = tx.clone();
//    pairs_in = pairs.clone();
//    thread::spawn(move|| poloniex::PoloniexMarketRunner{}.(tx_in, pairs_in));

    loop {
        thread::sleep(::std::time::Duration::from_secs(1));
        // TODO: prevent serializing this on every loop
        let hb = serde_json::to_string(&Broadcast::Heartbeat {}).unwrap();
        tx.send(hb).unwrap_or_else(|e| error!("Could not send heartbeat: {}", e));
    }
}

