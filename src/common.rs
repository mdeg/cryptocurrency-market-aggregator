pub type Timestamp = i64;
pub type Price = i64;
pub type Volume = i64;
pub type Total = i64;

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
        trades: Vec<(Timestamp, Price, Volume, Total)> //ts, price, volume, total
    },
    #[serde(rename = "trade")]
    Trade {
        source: Exchange,
        pair: CurrencyPair,
        trade: (Timestamp, Price, Volume, Total) //ts, price, volume, total
    },
    #[serde(rename = "connected")]
    Connected {
        multiplier: i32
    }
}

#[derive(Debug, Serialize)]
#[serde(rename_all = "lowercase")]
pub enum Exchange {
    BtcMarkets,
    Bitfinex
}

pub fn connect<T: ::ws::Factory + ConnectionFactory>(broadcast_tx: ::ws::Sender, pairs: Vec<CurrencyPair>) {
    ::std::thread::spawn(move || {
        let factory = T::new(broadcast_tx, pairs);
        let mut ws = ::ws::Builder::new().build(factory).unwrap();
        ws.connect(T::get_connect_addr()).unwrap();
        ws.run().unwrap();
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

//TODO: implement more pairs
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