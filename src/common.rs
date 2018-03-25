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
    #[serde(rename = "trade")]
    Trade {
        source: Exchange,
        pair: CurrencyPair,
        trades: Vec<(i64, i64, i64, i64)> //ts, price, volume, total
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

pub trait MarketRunner<Request, Response> {
    fn connect(broadcast_tx: ::ws::Sender, pairs: Vec<CurrencyPair>);

    fn map(&mut self, response: Response) -> Vec<Broadcast>;

    fn get_connect_addr() -> ::url::Url;

    fn get_requests(pairs: &[CurrencyPair]) -> Vec<Request>;

    fn stringify_pair(pair: &CurrencyPair) -> String;
}

// TODO: put this in a parsed external file
#[derive(Debug, Serialize, Copy, Clone)]
pub enum CurrencyPair {
    BTCXRP
}