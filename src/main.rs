extern crate url;
extern crate ws;
extern crate serde;
#[macro_use] extern crate serde_derive;
extern crate serde_json;
extern crate env_logger;
extern crate simplelog;
#[macro_use] extern crate log;
#[macro_use] extern crate dotenv_codegen;
extern crate dotenv;

// TODO: Poloniex support
//mod poloniex;
mod btcmarkets;
mod bitfinex;
mod common;
mod server;

use dotenv::dotenv;
use simplelog::*;

const MULTIPLIER: i32 = 100000000;

fn main() {

    dotenv().ok();

    let log_file = std::fs::File::create(dotenv!("LOG_FILE"))
        .expect("Could not create log file");

    CombinedLogger::init(vec![
        TermLogger::new(LevelFilter::Debug, Config::default()).expect("Could not initialise terminal logger"),
        WriteLogger::new(LevelFilter::Debug, Config::default(), log_file),
    ]).expect("Could not initialise combined logger");

    let pairs = common::CurrencyPair::parse(dotenv!("CURRENCY_PAIRS"));

    let server = server::Server::run();

//    common::connect::<bitfinex::BitfinexFactory>(server.tx(), pairs.clone());
    common::connect::<btcmarkets::BtcmarketsFactory>(server.tx(), pairs.clone());

    loop {
        ::std::thread::sleep(::std::time::Duration::from_secs(1));
        server.heartbeat();
    }
}

