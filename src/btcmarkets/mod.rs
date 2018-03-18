use common::{Broadcast, Exchange, MarketRunner};
use std::collections::HashMap;

type OrderbookElements = (Vec<OrderbookBid>, Vec<OrderbookAsk>);
type OrderbookBid = (i64, i64, i64); //price, amount, unknown (pair code?)
type OrderbookAsk = OrderbookBid;

pub struct BtcMarketsMarketRunner {
    orderbook_snapshots: HashMap<String, OrderbookElements>
}

impl BtcMarketsMarketRunner {
    pub fn new() -> Self {
        BtcMarketsMarketRunner {
            orderbook_snapshots: HashMap::new()
        }
    }

    fn map_orderbook_change(&mut self, bids: Vec<OrderbookBid>, asks: Vec<OrderbookBid>) -> Option<Broadcast> {
        // TODO
        let pair = ("XRP".to_string(), "BTC".to_string());
        let tmp_key = "XRPBTC".to_string();

        if !self.orderbook_snapshots.contains_key(&tmp_key) {
            self.orderbook_snapshots.insert(tmp_key, (bids, asks));

            return Some(Broadcast::OrderbookUpdate {
                seq_num: 1,
                source: Exchange::BtcMarkets,
                pair,
                bids: bids.into_iter().map(|(price, amount, _)| (price, amount)).collect(),
                asks: asks.into_iter().map(|(price, amount, _)| (price, amount)).collect()
            });
        }

        let last_snapshot = self.orderbook_snapshots.get(&tmp_key).unwrap();
        let (new_bids, new_asks) = Self::diff_snapshot(last_snapshot, bids, asks);
        self.orderbook_snapshots.insert(tmp_key, (bids, asks));

        if new_bids.is_empty() && new_asks.is_empty() {
            None
        } else {
            Some(Broadcast::OrderbookUpdate {
                seq_num: 1,
                source: Exchange::BtcMarkets,
                pair,
                bids: new_bids.into_iter().map(|(price, amount, _)| (price, amount)).collect(),
                asks: new_asks.into_iter().map(|(price, amount, _)| (price, amount)).collect()
            })
        }
    }

    fn diff_snapshot(last_snapshot: &OrderbookElements,
                     bids: Vec<OrderbookBid>, asks: Vec<OrderbookBid>) -> OrderbookElements {

        let tmp = "XRPBTC";

        let new_bids = bids.iter()
            .zip(last_snapshot.0)
            .take_while(|&(old, new)| *old != new)
            .map(|(_, new)| new)
            .collect();

        let new_asks = asks.iter()
            .zip(last_snapshot.1)
            .take_while(|&(old, new)| *old != new)
            .map(|(_, new)| new)
            .collect();

        (new_bids, new_asks)
    }
}

impl MarketRunner<Request, Response> for BtcMarketsMarketRunner {
    fn connect(&mut self, tx: ::ws::Sender, pairs: Vec<(String, String)>) {
        let requests = Self::get_requests(pairs);

        ::ws::connect(Self::get_connect_addr(), move |out| {
            info!("[BTCMarkets] Connected");

            requests.iter().for_each(|req| {
                match out.send(::serde_json::to_string(&req).unwrap()) {
                    Ok(_) => info!("[BTCMarkets] Sent {:?}", req),
                    Err(e) => error!("[BTCMarkets] Failed to send request: {}", e)
                }
            });

            // Need to clone this to send into FnMut closure
            let tx_in = tx.clone();
            move |msg: ::ws::Message| {
                debug!("[BTCMarkets] Raw message: {}", msg);

                match msg.into_text() {
                    Ok(txt) => {
                        match ::serde_json::from_str::<Response>(&txt) {
                            Ok(response) => {
                                if let Some(mapped) = self.map(response) {
                                    let serialized = ::serde_json::to_string(&mapped).unwrap();
                                    tx_in.send(serialized)
                                        .unwrap_or_else(|e| error!("[BTCMarkets] Could not broadcast: {}", e));
                                }
                            },
                            Err(e) => error!("[BTCMarkets] Could not deserialize message: {}", e)
                        }
                    },
                    Err(e) => error!("[BTCMarkets] Could not convert message to text: {}", e)
                }

                Ok(())
            }
        }).unwrap();
    }

    fn map(&mut self, response: Response) -> Option<Broadcast> {
        match response {
            Response::OrderbookChange { currency, instrument, bids, asks, .. } =>
                self.map_orderbook_change(bids, asks),

            Response::Trade { currency, instrument, trades, .. } => {
                Some (
                    Broadcast::Trade {
                        seq_num: 1,
                        source: Exchange::BtcMarkets,
                        pair: (currency, instrument),
                        trades
                    }
                )
            }
            _ => None
        }
    }

    fn get_connect_addr() -> &'static str {
        "ws://localhost:10001"
    }

    fn get_requests(pairs: Vec<(String, String)>) -> Vec<Request> {
        pairs.into_iter().flat_map(|(ref first, ref second)| {
            vec!(
                Request::JoinQueue {
                    channel_name: format!("Orderbook_{}{}", first, second),
                    event_name: "OrderBookChange".to_string()
                },
                Request::JoinQueue {
                    channel_name: format!("TRADE_{}{}", first, second),
                    event_name: "MarketTrade".to_string()
                }
            )
        }).collect()
    }
}

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
        bids: Vec<OrderbookBid>,
        asks: Vec<OrderbookAsk>
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
