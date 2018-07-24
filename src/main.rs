extern crate url;
extern crate ws;
extern crate serde;
#[macro_use] extern crate serde_derive;
extern crate serde_json;
extern crate simplelog;
#[macro_use] extern crate log;
#[macro_use] extern crate dotenv_codegen;
extern crate dotenv;
#[macro_use] extern crate error_chain;

mod domain;
mod broadcast_api;
mod consumer;

mod btcmarkets;
mod bitfinex;

use dotenv::dotenv;
use simplelog::*;
use std::fs::File;
use std::{thread, time};

const MULTIPLIER: i32 = 100_000_000;

fn main() {

    dotenv().ok();

    init_logger(dotenv!("LOG_FILE_PATH"));

    let pairs = domain::CurrencyPair::parse(dotenv!("CURRENCY_PAIRS"));

    let server = broadcast_api::server::Server::run();

    consumer::connect::<bitfinex::BitfinexFactory>(server.tx(), pairs.clone());
    consumer::connect::<btcmarkets::BtcmarketsFactory>(server.tx(), pairs.clone());

    loop {
        thread::sleep(time::Duration::from_secs(1));
        server.heartbeat();
    }
}

fn init_logger(path: &str) {
    let mut loggers: Vec<Box<SharedLogger>> = vec!();
    match File::create(path) {
        Ok(f) => loggers.push(WriteLogger::new(LevelFilter::Debug, Config::default(), f)),
        Err(e) => println!("Could not create log file at {}: {}", path, e)
    }
    match TermLogger::new(LevelFilter::Debug, Config::default()) {
        Some(logger) => loggers.push(logger),
        None => {
            println!("Could not create terminal logger: falling back to simple logger");
            loggers.push(SimpleLogger::new(LevelFilter::Debug, Config::default()));
        }
    }
    if let Err(e) = CombinedLogger::init(loggers) {
        println!("Could not initialise loggers: {}", e);
    }
}