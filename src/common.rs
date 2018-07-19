use std::{self, time};

pub type Timestamp = i64;
pub type Price = i64;
pub type Volume = i64;
pub type Total = i64;

// TODO: unify timestamps
#[derive(Debug, Serialize)]
pub enum Broadcast {
    #[serde(rename = "hb")]
    Heartbeat {},
    #[serde(rename = "orderbookUpdate")]
    OrderbookUpdate {
        source: Exchange,
        pair: CurrencyPair,
        bids: Vec<(Price, Volume)>,
        asks: Vec<(Price, Volume)>
    },
    #[serde(rename = "orderbookRemove")]
    OrderbookRemove {
        source: Exchange,
        pair: CurrencyPair,
        bids: Vec<(Price, Volume)>,
        asks: Vec<(Price, Volume)>
    },
    #[serde(rename = "orderbookSnapshot")]
    OrderbookSnapshot {
        source: Exchange,
        pair: CurrencyPair,
        bids: Vec<(Price, Volume)>,
        asks: Vec<(Price, Volume)>
    },
    #[serde(rename = "tradeSnapshot")]
    TradeSnapshot {
        source: Exchange,
        pair: CurrencyPair,
        trades: Vec<(Timestamp, Price, Volume, Total)>
    },
    #[serde(rename = "trade")]
    Trade {
        source: Exchange,
        pair: CurrencyPair,
        trade: (Timestamp, Price, Volume, Total)
    },
    #[serde(rename = "connected")]
    Connected {
        multiplier: i32
    },
    #[serde(rename = "connectionOpened")]
    ExchangeConnectionOpened {
        exchange: Exchange,
        ts: Timestamp
    },
    #[serde(rename = "connectionClosed")]
    ExchangeConnectionClosed {
        exchange: Exchange,
        ts: Timestamp
    }
}

#[derive(Debug)]
pub enum BroadcastType {
    None,
    One(Broadcast),
    Many(Vec<Broadcast>)
}

#[derive(Debug, Serialize)]
#[serde(rename_all = "lowercase")]
pub enum Exchange {
    BtcMarkets,
    Bitfinex
}

impl std::fmt::Display for Exchange {
    fn fmt(&self, f: &mut std::fmt::Formatter) -> Result<(), std::fmt::Error> {
        match self {
            Exchange::BtcMarkets => write!(f, "BTCMarkets"),
            Exchange::Bitfinex => write!(f, "Bitfinex")
        }
    }
}

pub fn connect<T: ::ws::Factory + ConnectionFactory>(broadcast_tx: ::ws::Sender, pairs: Vec<CurrencyPair>) {
    ::std::thread::spawn(move || {
        let factory = T::new(broadcast_tx, pairs);
        let mut ws = ::ws::Builder::new().build(factory).unwrap();
        ws.connect(T::get_connect_addr()).unwrap();
        ws.run().unwrap();
    });
}

pub fn timestamp() -> u64 {
    let time = time::SystemTime::now()
        .duration_since(time::UNIX_EPOCH)
        .expect("Time went backwards");

    time.as_secs() * 1000 + time.subsec_nanos() as u64 / 1_000_000
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

#[derive(Debug, Serialize, Copy, Clone, PartialEq, Ord, Eq, PartialOrd)]
pub enum CurrencyPair {
    XRPBTC
}

impl CurrencyPair {
    pub fn parse(values: &str) -> Vec<CurrencyPair> {
        let mut pairs: Vec<CurrencyPair> = values.split(',')
            .map(|x| Self::map(x).expect(&format!("Could not parse currency pair {}", x)))
            .collect();
        pairs.sort_unstable();
        pairs.dedup();
        pairs
    }

    pub fn map(value: &str) -> Option<CurrencyPair> {
        match value {
            "BTCXRP" => Some(CurrencyPair::XRPBTC),
            _ => None
        }
    }
}