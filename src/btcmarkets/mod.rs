mod api;

use self::api::*;
use common::{self, Broadcast, Exchange, ConnectionFactory, MarketHandler, CurrencyPair};
use std::collections::HashMap;
use std::io::Read;
use failure::Error;

type OrderbookEntry = (Price, Amount, i64);
type OrderbookBidsAndAsks = (Vec<OrderbookEntry>, Vec<OrderbookEntry>);

pub struct BtcmarketsHandler {
    orderbook_snapshots: HashMap<String, OrderbookBidsAndAsks>,
    broadcast_tx: ::ws::Sender,
    pairs: Vec<CurrencyPair>,
    out_tx: ::ws::Sender
}

// TODO: make a macro for this
impl ::ws::Handler for BtcmarketsHandler {
    fn on_open(&mut self, _: ::ws::Handshake) -> ::ws::Result<()> {
        info!("Connected to BTCMarkets");

        Self::get_requests(&self.pairs).into_iter().for_each(|req| {
            self.out_tx.send(req)
                .unwrap_or_else(|e| error!("Failed to send request: {}", e));
        });

        Ok(())
    }

    fn on_message(&mut self, msg: ::ws::Message) -> ::ws::Result<()> {
        debug!("Raw message: {}", msg);

        match msg.into_text() {
            Ok(txt) => {
                match ::serde_json::from_str::<Response>(&txt) {
                    Ok(response) => {
                        map(response, &mut self.orderbook_snapshots).into_iter()
                            .map(|r| ::serde_json::to_string(&r).unwrap())
                            .for_each(|msg| {
                                self.broadcast_tx.send(msg)
                                    .unwrap_or_else(|e| error!("Could not broadcast: {}", e));
                            });
                    },
                    Err(e) => error!("Could not deserialize message: {}", e)
                }
            },
            Err(e) => error!("Could not convert message to text: {}", e)
        }

        Ok(())
    }
}


impl MarketHandler for BtcmarketsHandler {

    fn get_requests(pairs: &[CurrencyPair]) -> Vec<String> {
        pairs.iter().flat_map(|currency_pair| {
            let pair = Self::stringify_pair(currency_pair);
            vec!(
                Request::JoinQueue {
                    channel_name: format!("Orderbook_{}", pair),
                    event_name: "OrderBookChange".to_string()
                },
                Request::JoinQueue {
                    channel_name: format!("TRADE_{}", pair),
                    event_name: "MarketTrade".to_string()
                }
            )}).map(|req| ::serde_json::to_string(&req).unwrap()).collect()
    }

    fn stringify_pair(pair: &CurrencyPair) -> String {
        match *pair {
            CurrencyPair::XRPBTC => "XRPBTC"
        }.to_string()
    }
}

pub struct BtcmarketsFactory {
    broadcast_tx: ::ws::Sender,
    pairs: Vec<CurrencyPair>,
}

impl ConnectionFactory for BtcmarketsFactory {

    fn new(broadcast_tx: ::ws::Sender, pairs: Vec<CurrencyPair>) -> Self {
        Self { broadcast_tx, pairs }
    }

    fn get_connect_addr() -> ::url::Url {
        ::url::Url::parse(dotenv!("BTCMARKETS_ADDR")).unwrap()
    }
}

impl ::ws::Factory for BtcmarketsFactory {
    type Handler = BtcmarketsHandler;

    fn connection_made(&mut self, sender: ::ws::Sender) -> Self::Handler {
        BtcmarketsHandler {
            orderbook_snapshots: HashMap::new(),
            broadcast_tx: self.broadcast_tx.clone(),
            pairs: self.pairs.clone(),
            out_tx: sender
        }
    }
}

fn map(response: Response, orderbook_snapshots: &mut HashMap<String, OrderbookBidsAndAsks>) -> Vec<Broadcast> {
    match response {
        Response::OrderbookChange { currency, instrument, bids, asks, .. } => {
            let pair = map_pair_code(&instrument, &currency);
            map_orderbook_change(orderbook_snapshots, pair, bids, asks)
        },
        Response::Trade { currency, instrument, trades, .. } => {
            let pair = map_pair_code(&instrument, &currency);
            vec!(Broadcast::TradeSnapshot {
                source: Exchange::BtcMarkets,
                pair,
                trades
            })},
        _ => vec!()
    }
}

// Pairs list: https://api.btcmarkets.net/v2/market/active
// TODO: build pairs from API call
fn map_pair_code(instrument: &str, currency: &str) -> CurrencyPair {
    match format!("{}{}", instrument, currency).as_str() {
        "XRPBTC" => CurrencyPair::XRPBTC,
        _ => panic!("Could not map pair code {}{}", instrument, currency)
    }
}

fn map_orderbook_change(orderbook_snapshots: &mut HashMap<String, OrderbookBidsAndAsks>, pair: CurrencyPair,
                        bids: Vec<OrderbookEntry>, asks: Vec<OrderbookEntry>) -> Vec<Broadcast> {
    let key = BtcmarketsHandler::stringify_pair(&pair);

    if !orderbook_snapshots.contains_key(&key) {
        orderbook_snapshots.insert(key, (bids.clone(), asks.clone()));

        return vec!(Broadcast::OrderbookSnapshot {
            source: Exchange::BtcMarkets,
            pair,
            bids: bids.into_iter().map(|(price, amount, _)| (price, amount)).collect(),
            asks: asks.into_iter().map(|(price, amount, _)| (price, amount)).collect()
        });
    }

    let last_snapshot = orderbook_snapshots.get_mut(&key).unwrap();

    let (removed_bids, new_bids) = diff(&last_snapshot.0, &bids);
    let (removed_asks, new_asks) = diff(&last_snapshot.1, &asks);
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

// TODO: do this better
fn diff(first: &Vec<OrderbookEntry>, second: &Vec<OrderbookEntry>) -> (Vec<OrderbookEntry>, Vec<OrderbookEntry>) {
    (first.clone().into_iter().filter(|&x| !second.contains(&x)).collect(),
     second.clone().into_iter().filter(|&x| !first.contains(&x)).collect())
}

fn retrieve_currency_list() -> Result<Vec<CurrencyPair>, Error> {
    let mut res = ::reqwest::get(dotenv!("BTCMARKETS_PAIR_LIST_ADDR"))?;
    let mut body = String::new();
    res.read_to_string(&mut body)?;

    let pair_list = ::serde_json::from_str::<CurrencyPairList>(&body)?;

    Ok(pair_list.markets
        .into_iter()
        .filter_map(|api_pair| { map_to_currency_pair(&api_pair.instrument, &api_pair.currency) })
        .collect()
    )
}

fn map_to_currency_pair(instrument: &str, currency: &str) -> Option<common::CurrencyPair> {
    match (instrument, currency) {
        ("BTC", "XRP") => Some(CurrencyPair::XRPBTC),
        _ => {
            error!("Did not match pair: instrument {} currency {}", instrument, currency);
            None
        }
    }
}