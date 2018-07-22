pub type Timestamp = i64;
pub type Price = i64;
pub type Volume = i64;
pub type Total = i64;
pub type Amount = i64;
pub type PairCode = i64;

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
    OrderbookSnapshot {
        currency: String,
        instrument: String,
        timestamp: Timestamp,
        #[serde(rename = "marketId")]
        market_id: i64,
        #[serde(rename = "snapshotId")]
        snapshot_id: i64,
        bids: Vec<(Price, Amount, PairCode)>,
        asks: Vec<(Price, Amount, PairCode)>
    },
    Trade {
        id: i64,
        timestamp: Timestamp,
        #[serde(rename = "marketId")]
        market_id: i64,
        agency: String,
        instrument: String,
        currency: String,
        trades: Vec<(Timestamp, Price, Volume, Total)>
    },
    Status {
        status: String
    }
}
