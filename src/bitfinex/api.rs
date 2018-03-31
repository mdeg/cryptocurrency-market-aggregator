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
        channel_id: i32,
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
    Heartbeat(i32, String), //channelid, string="hb"
    InitialTrade(i32, Vec<(i64, f64, f64, f64)>), //channelId, (orderId, ts, amount, price)
    InitialOrderbook(i32, Vec<(i64, f64, f64)>), //channelId, (orderid, price, amount)
    OrderbookUpdate(i32, (i64, f64, f64)), //channelId, (orderid, price, amount)
    Trade(i32, String, (i64, f64, f64, f64)) //channelId, te/tu (?), (id, ts, amount, price)
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