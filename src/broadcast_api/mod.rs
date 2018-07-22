pub mod server;

use common::{Exchange, CurrencyPair};

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