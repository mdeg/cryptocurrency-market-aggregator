use btcmarkets::OrderbookEntry;

#[derive(Debug, Serialize)]
#[serde(untagged)]
pub enum Request {
    JoinQueue {
        #[serde(rename = "channelName")]
        channel_name: String,
        #[serde(rename = "eventName")]
        event_name: String
    }
}

#[derive(Debug, Deserialize)]
#[serde(untagged)]
pub enum Response {
    OrderbookChange {
        currency: String,
        instrument: String,
        timestamp: i64,
        #[serde(rename = "marketId")]
        market_id: i64,
        #[serde(rename = "snapshotId")]
        snapshot_id: i64,
        bids: Vec<OrderbookEntry>,
        asks: Vec<OrderbookEntry>
    },
    Trade {
        id: i64,
        timestamp: i64,
        #[serde(rename = "marketId")]
        market_id: i64,
        agency: String,
        instrument: String,
        currency: String,
        trades: Vec<(i64, i64, i64, i64)> //ts, price, volume, total
    },
    Status {
        status: String
    },
}
