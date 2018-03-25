use common::{Broadcast, Exchange, CurrencyPair, MarketRunner};
use std::collections::HashMap;

pub type OrderbookEntry = (i64, i64, i64); //price, amount, unknown (pair code?)
type OrderbookBidsAndAsks = (Vec<OrderbookEntry>, Vec<OrderbookEntry>);

pub struct BtcMarketsMarketRunner {
    orderbook_snapshots: HashMap<String, OrderbookBidsAndAsks>,
    broadcast_tx: ::ws::Sender,
    pairs: Vec<CurrencyPair>,
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
                        self.map(response).into_iter()
                            .map(|r| ::serde_json::to_string(&r).unwrap())
                            .for_each(|msg| {
                                self.broadcast_tx.send(msg)
                                    .unwrap_or_else(|e| error!("[BTCMarkets] Could not broadcast: {}", e));
                            });
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

    fn map_orderbook_change(&mut self, pair: CurrencyPair, bids: Vec<OrderbookEntry>, asks: Vec<OrderbookEntry>) -> Vec<Broadcast> {
        let key = Self::stringify_pair(&pair);

        if !self.orderbook_snapshots.contains_key(&key) {
            self.orderbook_snapshots.insert(key, (bids.clone(), asks.clone()));

            return vec!(Broadcast::OrderbookUpdate {
                source: Exchange::BtcMarkets,
                pair,
                bids: bids.into_iter().map(|(price, amount, _)| (price, amount)).collect(),
                asks: asks.into_iter().map(|(price, amount, _)| (price, amount)).collect()
            });
        }

        let last_snapshot = self.orderbook_snapshots.get_mut(&key).unwrap();

        let (removed_bids, new_bids) = Self::diff(&last_snapshot.0, &bids);
        let (removed_asks, new_asks) = Self::diff(&last_snapshot.1, &asks);
        *last_snapshot = (bids, asks);

        let mut responses = vec!();

        if !removed_bids.is_empty() || !removed_asks.is_empty() {
            responses.push(Broadcast::OrderbookRemove {
                source: Exchange::BtcMarkets,
                pair,
                bids: removed_bids.into_iter().map(|(price, amount, _)| (price, amount)).collect(),
                asks: removed_asks.into_iter().map(|(price, amount, _)| (price, amount)).collect()
            });
        }

        if !new_bids.is_empty() || !new_asks.is_empty() {
            responses.push(Broadcast::OrderbookUpdate {
                source: Exchange::BtcMarkets,
                pair,
                bids: new_bids.into_iter().map(|(price, amount, _)| (price, amount)).collect(),
                asks: new_asks.into_iter().map(|(price, amount, _)| (price, amount)).collect()
            });
        }

        responses
    }

    fn diff(first: &Vec<OrderbookEntry>, second: &Vec<OrderbookEntry>) -> (Vec<OrderbookEntry>, Vec<OrderbookEntry>) {
        (first.clone().into_iter().filter(|&x| !second.contains(&x)).collect(),
        second.clone().into_iter().filter(|&x| !first.contains(&x)).collect())
    }
}

struct BtcMarketsRunnerWsFactory {
    broadcast_tx: ::ws::Sender,
    pairs: Vec<CurrencyPair>,
}

impl ::ws::Factory for BtcMarketsRunnerWsFactory {
    type Handler = BtcMarketsMarketRunner;

    fn connection_made(&mut self, sender: ::ws::Sender) -> Self::Handler {
        BtcMarketsMarketRunner {
            orderbook_snapshots: HashMap::new(),
            broadcast_tx: self.broadcast_tx.clone(),
            pairs: self.pairs.clone(),
            out_tx: sender
        }
    }
}

impl MarketRunner<Request, Response> for BtcMarketsMarketRunner {
    fn connect(broadcast_tx: ::ws::Sender, pairs: Vec<CurrencyPair>) {
        let factory = BtcMarketsRunnerWsFactory { broadcast_tx, pairs };
        let mut ws = ::ws::Builder::new().build(factory).unwrap();
        ws.connect(Self::get_connect_addr()).unwrap();
        ws.run().unwrap();
    }

    fn map(&mut self, response: Response) -> Vec<Broadcast> {
        match response {
            Response::OrderbookChange { currency, instrument, bids, asks, .. } => {
                //TODO: convert pair
                self.map_orderbook_change(CurrencyPair::BTCXRP, bids, asks)
            },
            Response::Trade { currency, instrument, trades, .. } =>
                vec!(Broadcast::Trade {
                        source: Exchange::BtcMarkets,
                        // TODO: convert pair
                        pair: CurrencyPair::BTCXRP,
                        trades
                    }),
            _ => vec!()
        }
    }

    fn get_connect_addr() -> ::url::Url {
        ::url::Url::parse("ws://localhost:10001").unwrap()
    }

    fn get_requests(pairs: &[CurrencyPair]) -> Vec<Request> {
        pairs.iter().flat_map(|ref currency_pair| {
            let pair = Self::stringify_pair(*currency_pair);
            vec!(
                Request::JoinQueue {
                    channel_name: format!("Orderbook_{}", pair),
                    event_name: "OrderBookChange".to_string()
                },
                Request::JoinQueue {
                    channel_name: format!("TRADE_{}", pair),
                    event_name: "MarketTrade".to_string()
                }
            )
        }).collect()
    }

    fn stringify_pair(pair: &CurrencyPair) -> String {
        match *pair {
            CurrencyPair::BTCXRP => "XRPBTC"
        }.to_string()
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
