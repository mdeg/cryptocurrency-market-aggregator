use common::{Broadcast, Exchange, MarketRunner};
use std::collections::HashMap;

pub type OrderbookBid = (i64, i64, i64); //price, amount, unknown (pair code?)
pub type OrderbookAsk = OrderbookBid;
type OrderbookBidsAndAsks = (Vec<OrderbookBid>, Vec<OrderbookAsk>);

pub struct BtcMarketsMarketRunner {
    orderbook_snapshots: HashMap<String, OrderbookBidsAndAsks>,
    seq_num: i64,
    broadcast_tx: ::ws::Sender,
    pairs: Vec<(String, String)>,
    out_tx: ::ws::Sender
}

impl ::ws::Handler for BtcMarketsMarketRunner {
    fn on_open(&mut self, _: ::ws::Handshake) -> ::ws::Result<()> {
        info!("[BTCMarkets] Connected");

        BtcMarketsMarketRunner::get_requests(&self.pairs).iter().for_each(|req| {
            match self.out_tx.send(::serde_json::to_string(&req).unwrap()) {
                Ok(_) => info!("[BTCMarkets] Sent {:?}", req),
                Err(e) => error!("[BTCMarkets] Failed to send request: {}", e)
            }
        });

        Ok(())
    }

    fn on_message(&mut self, msg: ::ws::Message) -> ::ws::Result<()> {
        debug!("[BTCMarkets] Raw message: {}", msg);

        match msg.into_text() {
            Ok(txt) => {
                match ::serde_json::from_str::<Response>(&txt) {
                    Ok(response) => {
                        if let Some(mapped) = self.map(response) {
                            let serialized = ::serde_json::to_string(&mapped).unwrap();
                            self.broadcast_tx.send(serialized)
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
}

impl BtcMarketsMarketRunner {

    fn map_orderbook_change(&mut self, bids: Vec<OrderbookBid>, asks: Vec<OrderbookBid>) -> Option<Broadcast> {
        // TODO
        let pair = ("XRP".to_string(), "BTC".to_string());
        let tmp_key = "XRPBTC".to_string();

        if !self.orderbook_snapshots.contains_key(&tmp_key) {
            println!("Initial orderbook");

            self.orderbook_snapshots.insert(tmp_key, (bids.clone(), asks.clone()));
            self.seq_num += 1;

            return Some(Broadcast::OrderbookUpdate {
                seq_num: self.seq_num,
                source: Exchange::BtcMarkets,
                pair,
                bids: bids.into_iter().map(|(price, amount, _)| (price, amount)).collect(),
                asks: asks.into_iter().map(|(price, amount, _)| (price, amount)).collect()
            });
        }

        let last_snapshot = self.orderbook_snapshots.get_mut(&tmp_key).unwrap();
        let (new_bids, new_asks) = Self::diff_snapshot(last_snapshot, &bids, &asks);
        *last_snapshot = (bids, asks);

        if new_bids.is_empty() && new_asks.is_empty() {
            println!("No new orderbook updates");
            None
        } else {
            println!("New stuff to print");

            self.seq_num += 1;

            Some(Broadcast::OrderbookUpdate {
                seq_num: self.seq_num,
                source: Exchange::BtcMarkets,
                pair,
                bids: new_bids.into_iter().map(|(price, amount, _)| (price, amount)).collect(),
                asks: new_asks.into_iter().map(|(price, amount, _)| (price, amount)).collect()
            })
        }
    }

    // Orderbook lists the top 50 bids and top 50 asks at the current time ordered by price
    // Need to diff the snapshots to de-duplicate multiple bids
    // TODO: proper filtering system: orderbook updates, orderbook removes
    fn diff_snapshot(last_snapshot: &OrderbookBidsAndAsks,
                     bids: &[OrderbookBid], asks: &[OrderbookBid]) -> OrderbookBidsAndAsks {

        let tmp = "XRPBTC";

        let new_bids = bids.iter()
            .zip(&last_snapshot.0)
            .take_while(|&(new, old)| *old != *new)
            .map(|(new, _)| *new)
            .inspect(|x| println!("debug new bid: {:?}", x))
            .collect();

        let new_asks = asks.iter()
            .zip(&last_snapshot.1)
            .take_while(|&(new, old)| *old != *new)
            .map(|(new, _)| *new)
            .inspect(|x| println!("debug new ask: {:?}", x))
            .collect();

        println!("new bids: {:?}", new_bids);
        println!("new_asks: {:?}", new_asks);

        (new_bids, new_asks)
    }
}

struct BtcMarketsRunnerWsFactory {
    broadcast_tx: ::ws::Sender,
    pairs: Vec<(String, String)>,
}

impl ::ws::Factory for BtcMarketsRunnerWsFactory {
    type Handler = BtcMarketsMarketRunner;

    fn connection_made(&mut self, sender: ::ws::Sender) -> Self::Handler {
        BtcMarketsMarketRunner {
            orderbook_snapshots: HashMap::new(),
            broadcast_tx: self.broadcast_tx.clone(),
            pairs: self.pairs.clone(),
            out_tx: sender,
            seq_num: 0
        }
    }
}

impl MarketRunner<Request, Response> for BtcMarketsMarketRunner {
    fn connect(broadcast_tx: ::ws::Sender, pairs: Vec<(String, String)>) {
        let factory = BtcMarketsRunnerWsFactory { broadcast_tx, pairs };
        let mut ws = ::ws::Builder::new().build(factory).unwrap();
        ws.connect(Self::get_connect_addr()).unwrap();
        ws.run().unwrap();
    }

    fn map(&mut self, response: Response) -> Option<Broadcast> {
        match response {
            Response::OrderbookChange { currency, instrument, bids, asks, .. } =>
                self.map_orderbook_change(bids, asks),

            Response::Trade { currency, instrument, trades, .. } => {
                self.seq_num += 1;
                Some (
                    Broadcast::Trade {
                        seq_num: self.seq_num,
                        source: Exchange::BtcMarkets,
                        pair: (currency, instrument),
                        trades
                    }
                )
            }
            _ => None
        }
    }

    fn get_connect_addr() -> ::url::Url {
        ::url::Url::parse("ws://localhost:10001").unwrap()
    }

    fn get_requests(pairs: &[(String, String)]) -> Vec<Request> {
        pairs.iter().flat_map(|&(ref first, ref second)| {
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
