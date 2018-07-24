pub mod server;

use super::domain::*;

pub type Timestamp = i64;
pub type Price = i64;
pub type Volume = i64;
pub type Total = i64;

#[derive(Debug, Serialize)]
#[serde(rename_all = "camelCase")]
pub enum Broadcast {
    #[serde(rename = "hb")]
    Heartbeat {},
    OrderbookUpdate {
        source: Exchange,
        pair: CurrencyPair,
        bids: Vec<(Price, Volume)>,
        asks: Vec<(Price, Volume)>
    },
    OrderbookRemove {
        source: Exchange,
        pair: CurrencyPair,
        bids: Vec<(Price, Volume)>,
        asks: Vec<(Price, Volume)>
    },
    OrderbookSnapshot {
        source: Exchange,
        pair: CurrencyPair,
        bids: Vec<(Price, Volume)>,
        asks: Vec<(Price, Volume)>
    },
    TradeSnapshot {
        source: Exchange,
        pair: CurrencyPair,
        trades: Vec<(Timestamp, Price, Volume, Total)>
    },
    Trade {
        source: Exchange,
        pair: CurrencyPair,
        trade: (Timestamp, Price, Volume, Total)
    },
    Connected {
        multiplier: i32
    },
    ExchangeConnectionOpened {
        exchange: Exchange,
        ts: Timestamp
    },
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