pub type ChannelId = i32;
pub type OrderId = i64;
pub type Timestamp = f64;
pub type Amount = f64;
pub type Price = f64;

#[derive(Debug, Deserialize)]
#[serde(untagged)]
pub enum Response {
    Connected {
        event: String,
        version: i32,
        #[serde(skip)]
        platform: ::serde::de::IgnoredAny
    },
    SubscribeConfirmation {
        event: String,
        channel: String,
        #[serde(rename = "chanId")]
        channel_id: ChannelId,
        pair: String,
        symbol: String,
        #[serde(rename = "prec")]
        precision: Option<Precision>,
        #[serde(rename = "freq")]
        frequency: Option<Frequency>,
        #[serde(rename = "len")]
        length: Option<String>,
    },
    SubscribeError {
        event: String,
        channel: String,
        code: i32,
        msg: String,
        pair: String,
        symbol: String,
        #[serde(rename = "prec")]
        precision: Precision,
        #[serde(rename = "freq")]
        frequency: Frequency,
        #[serde(rename = "len")]
        length: String,
    },
    Heartbeat(ChannelId, String), // The string here is always "hb"
    InitialTrade(ChannelId, Vec<(OrderId, Timestamp, Amount, Price)>),
    Trade(ChannelId, TradeUpdateType, (OrderId, Timestamp, Amount, Price)),
    InitialOrderbook(ChannelId, Vec<(OrderId, Price, Amount)>),
    OrderbookUpdate(ChannelId, (OrderId, Price, Amount)),
}

#[derive(Debug, Serialize)]
#[serde(untagged)]
pub enum Request {
    JoinQueue {
        event: String,
        channel: String,
        symbol: String,
        #[serde(rename = "prec")]
        precision: Precision,
        #[serde(rename = "freq")]
        frequency: Frequency,
        #[serde(rename = "len")]
        length: String
    }
}

#[derive(Debug, Deserialize, Serialize)]
pub enum Precision {
    R0,
    P0,
    P1,
    P2,
    P3
}

#[derive(Debug, Deserialize, Serialize)]
pub enum Frequency {
    F0,
    F1
}

#[derive(Debug, Deserialize, Serialize)]
pub enum TradeUpdateType {
    WithTradeId,
    WithoutTradeId
}