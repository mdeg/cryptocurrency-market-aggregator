mod api;

use common::{Broadcast, Exchange, ConnectionFactory, MarketHandler, CurrencyPair};
use std::collections::HashMap;

pub struct BitfinexHandler {
    broadcast_tx: ::ws::Sender,
    pairs: Vec<CurrencyPair>,
    out_tx: ::ws::Sender,
    state: State
}

impl ::ws::Handler for BitfinexHandler {
    fn on_open(&mut self, _: ::ws::Handshake) -> ::ws::Result<()> {
        info!("Connected");

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
                match ::serde_json::from_str::<api::Response>(&txt) {
                    Ok(response) => {
                        map(response, &mut self.state).into_iter()
                            .map(|r| ::serde_json::to_string(&r).unwrap())
                            .for_each(|msg| self.broadcast_tx.send(msg)
                                .unwrap_or_else(|e| error!("Could not broadcast: {}", e)))
                    },
                    Err(e) => error!("Could not deserialize message: {}", e)
                }
            },
            Err(e) => error!("Could not convert message to text: {}", e)
        };

        Ok(())
    }
}

impl MarketHandler for BitfinexHandler {

    fn get_requests(pairs: &[CurrencyPair]) -> Vec<String> {
        pairs.iter().flat_map(|ref pair| { vec!(
            api::Request::JoinQueue {
                event: "subscribe".to_string(),
                channel: "book".to_string(),
                symbol: Self::stringify_pair(*pair),
                precision: api::Precision::R0,
                frequency: api::Frequency::F0,
                length: 100.to_string()
            },
            api::Request::JoinQueue {
                event: "subscribe".to_string(),
                channel: "trades".to_string(),
                symbol: Self::stringify_pair(*pair),
                precision: api::Precision::R0,
                frequency: api::Frequency::F0,
                length: 100.to_string()
            }
        )}).map(|req| ::serde_json::to_string(&req).unwrap()).collect()
    }

    fn stringify_pair(pair: &CurrencyPair) -> String {
        match *pair {
            CurrencyPair::XRPBTC => "tXRPBTC"
        }.to_string()
    }
}

pub struct BitfinexFactory {
    broadcast_tx: ::ws::Sender,
    pairs: Vec<CurrencyPair>
}

impl ConnectionFactory for BitfinexFactory {
    fn new(broadcast_tx: ::ws::Sender, pairs: Vec<CurrencyPair>) -> Self {
        Self { broadcast_tx, pairs }
    }

    fn get_connect_addr() -> ::url::Url {
        ::url::Url::parse(dotenv!("BITFINEX_ADDR")).unwrap()
    }
}

impl ::ws::Factory for BitfinexFactory {
    type Handler = BitfinexHandler;

    fn connection_made(&mut self, sender: ::ws::Sender) -> Self::Handler {
        Self::Handler {
            broadcast_tx: self.broadcast_tx.clone(),
            pairs: self.pairs.clone(),
            out_tx: sender,
            state: State::default()
        }
    }
}

struct State {
    channels: HashMap<i32, CurrencyPair>
}

impl Default for State {
    fn default() -> Self {
        Self {
            channels: HashMap::new()
        }
    }
}

fn map(response: api::Response, state: &mut State) -> Vec<Broadcast> {
    match response {
        api::Response::OrderbookUpdate(channel_id, (_, price, amount)) => {
            let pair = state.channels.get(&channel_id).expect("Could not find channel ID");
            map_orderbook_update(*pair, price, amount)
        },
        api::Response::Trade(channel_id, unknown_string, trades) => {
            let pair = state.channels.get(&channel_id).expect("Could not find channel ID");
            map_trade(*pair, trades)
        },
        api::Response::SubscribeConfirmation { channel_id, symbol, .. } => {
            let pair = map_pair_code(&symbol);
            state.channels.insert(channel_id, pair);
            vec!()
        }
        // TODO: map initial responses
        _ => vec!()
    }
}

// Pairs list: https://api.bitfinex.com/v1/symbols
fn map_pair_code(pair_code: &str) -> CurrencyPair {
    match pair_code {
        "tXRPBTC" => CurrencyPair::XRPBTC,
        _ => panic!("Could not map pair code {}", pair_code)
    }
}

fn map_orderbook_update(pair: CurrencyPair, price: f64, amount: f64) -> Vec<Broadcast> {
    let conv_price = (price * ::MULTIPLIER as f64) as i64;
    let conv_amount = (amount * ::MULTIPLIER as f64) as i64;

    let (mut bids, mut asks) = (vec!(), vec!());
    //TODO: trading vs funding?
    if conv_amount > 0 {
        bids.push((conv_price, conv_amount));
    } else {
        asks.push((conv_price, conv_amount.abs()));
    }

    if conv_price == 0 {
        vec!(Broadcast::OrderbookRemove {
            source: Exchange::Bitfinex,
            pair,
            bids,
            asks
        })
    } else {
        vec!(Broadcast::OrderbookUpdate {
            source: Exchange::Bitfinex,
            pair,
            bids,
            asks
        })
    }
}

fn map_trade(pair: CurrencyPair, trade: (i64, f64, f64, f64)) -> Vec<Broadcast> {
    let (order_id, ts, amount, price) = trade;
    let conv_price = (price * ::MULTIPLIER as f64) as i64;
    let conv_amount = (amount * ::MULTIPLIER as f64) as i64;

    vec!(Broadcast::Trade {
        source: Exchange::Bitfinex,
        pair,
        trade: (ts as i64, conv_price, conv_amount, conv_price * conv_amount)
    })
}

fn map_initial_trade(pair: CurrencyPair, trades: Vec<(i64, f64, f64, f64)>) -> Vec<Broadcast> {
    let trades_out = trades.into_iter().map(|(order_id, ts, amount, price)| {
        let conv_price = (price * ::MULTIPLIER as f64) as i64;
        let conv_amount = (amount * ::MULTIPLIER as f64) as i64;

        (ts as i64, conv_price, conv_amount, conv_price * conv_amount)
    }).collect();

    vec!(Broadcast::TradeSnapshot {
        source: Exchange::Bitfinex,
        pair,
        trades: trades_out
    })
}