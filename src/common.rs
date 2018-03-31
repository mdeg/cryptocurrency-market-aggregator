#[derive(Debug, Serialize)]
pub enum Broadcast {
    #[serde(rename = "hb")]
    Heartbeat {},
    #[serde(rename = "orderbookUpdate")]
    OrderbookUpdate {
        source: Exchange,
        pair: CurrencyPair,
        bids: Vec<(i64, i64)>,
        asks: Vec<(i64, i64)>
    },
    #[serde(rename = "orderbookRemove")]
    OrderbookRemove {
        source: Exchange,
        pair: CurrencyPair,
        bids: Vec<(i64, i64)>,
        asks: Vec<(i64, i64)>
    },
    #[serde(rename = "tradeSnapshot")]
    TradeSnapshot {
        source: Exchange,
        pair: CurrencyPair,
        trades: Vec<(i64, i64, i64, i64)> //ts, price, volume, total
    },
    #[serde(rename = "trade")]
    Trade {
        source: Exchange,
        pair: CurrencyPair,
        trade: (i64, i64, i64, i64) //ts, price, volume, total
    },
    #[serde(rename = "connected")]
    Connected {
        multiplier: i32
    }
}

#[derive(Debug, Serialize)]
pub enum Exchange {
    #[serde(rename = "btcmarkets")]
    BtcMarkets,
    #[serde(rename = "poloniex")]
    Poloniex,
    #[serde(rename = "bitfinex")]
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

// TODO: put this in a parsed external file
#[derive(Debug, Serialize, Copy, Clone)]
pub enum CurrencyPair {
    BTCXRP
}