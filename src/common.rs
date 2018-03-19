#[derive(Debug, Serialize)]
pub enum Broadcast {
    #[serde(rename = "hb")]
    Heartbeat {},
    #[serde(rename = "orderbookUpdate")]
    OrderbookUpdate {
        seq_num: i64,
        source: Exchange,
        pair: (String, String),
        bids: Vec<(i64, i64)>,
        asks: Vec<(i64, i64)>
    },
    #[serde(rename = "orderbookRemove")]
    OrderbookRemove {
        seq_num: i64,
        source: Exchange,
        pair: (String, String),
        bids: Vec<(i64, i64)>,
        asks: Vec<(i64, i64)>
    },
    #[serde(rename = "trade")]
    Trade {
        seq_num: i64,
        source: Exchange,
        pair: (String, String),
        trades: Vec<(i64, i64, i64, i64)>
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
    fn connect(broadcast_tx: ::ws::Sender, pairs: Vec<(String, String)>);

    fn map(&mut self, response: Response) -> Vec<Broadcast>;

    fn get_connect_addr() -> ::url::Url;

    fn get_requests(pairs: &[(String, String)]) -> Vec<Request>;
}