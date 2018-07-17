mod api;

use self::api::*;
use common::{self, Broadcast, Exchange, ConnectionFactory, MarketHandler, CurrencyPair};
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
                match ::serde_json::from_str::<Response>(&txt) {
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
        pairs.iter().flat_map(|pair| { vec!(
            Request::JoinQueue {
                event: "subscribe".to_string(),
                channel: "book".to_string(),
                symbol: Self::stringify_pair(pair),
                precision: Precision::R0,
                frequency: Frequency::F0,
                length: 100.to_string()
            },
            Request::JoinQueue {
                event: "subscribe".to_string(),
                channel: "trades".to_string(),
                symbol: Self::stringify_pair(pair),
                precision: Precision::R0,
                frequency: Frequency::F0,
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

fn map(response: Response, state: &mut State) -> Vec<Broadcast> {
    match response {
        Response::OrderbookUpdate(channel_id, (_, price, amount)) => {
            let pair = state.channels.get(&channel_id).expect(
                &format!("Could not find channel ID {}", channel_id));
            map_orderbook_update(*pair, price, amount)
        },
        Response::Trade(channel_id, _trade_update_type, trades) => {
            let pair = state.channels.get(&channel_id).expect(
                &format!("Could not find channel ID {}", channel_id));
            map_trade(*pair, trades)
        },
        Response::SubscribeConfirmation { channel_id, symbol, .. } => {
            let pair = map_pair_code(&symbol);
            state.channels.insert(channel_id, pair);
            vec!()
        },
        Response::InitialOrderbook(channel_id, orders) => {
            let pair = state.channels.get(&channel_id).expect(
                &format!("Could not find channel ID {}", channel_id));
            map_initial_orderbook(*pair, orders)
        },
        Response::InitialTrade(channel_id, trades) => {
            let pair = state.channels.get(&channel_id).expect(
                &format!("Could not find channel ID {}", channel_id));
            map_initial_trades(*pair, trades)
        }
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
    let standardised_price = common::standardise_value(price);
    let standardised_amount = common::standardise_value(amount);

    let (mut bids, mut asks) = (vec!(), vec!());
    if standardised_amount > 0 {
        bids.push((standardised_price, standardised_amount));
    } else {
        asks.push((standardised_price, standardised_amount.abs()));
    }

    if standardised_price == 0 {
        vec!(Broadcast::OrderbookRemove {
            source: Exchange::Bitfinex,
            pair, bids, asks
        })
    } else {
        vec!(Broadcast::OrderbookUpdate {
            source: Exchange::Bitfinex,
            pair, bids, asks
        })
    }
}

fn map_trade(pair: CurrencyPair, trade: (OrderId, Timestamp, Amount, Price)) -> Vec<Broadcast> {
    let (_order_id, ts, amount, price) = trade;
    let standardised_price = common::standardise_value(price);
    let standardised_amount = common::standardise_value(amount);

    vec!(Broadcast::Trade {
        source: Exchange::Bitfinex,
        pair,
        trade: (ts as i64, standardised_price, standardised_amount, standardised_price * standardised_amount)
    })
}

fn map_initial_trades(pair: CurrencyPair, trades: Vec<(OrderId, Timestamp, Amount, Price)>) -> Vec<Broadcast> {
    let trades_out = trades.into_iter().map(|(_order_id, ts, amount, price)| {
        let standardised_price = common::standardise_value(price);
        let standardised_amount = common::standardise_value(amount);

        (ts as i64, standardised_price, standardised_amount, standardised_price * standardised_amount)
    }).collect();

    vec!(Broadcast::TradeSnapshot {
        source: Exchange::Bitfinex,
        pair,
        trades: trades_out
    })
}

fn map_initial_orderbook(pair: CurrencyPair, orders: Vec<(OrderId, Price, Amount)>) -> Vec<Broadcast> {
    let (mut bids, mut asks) = (vec!(), vec!());
    for order in orders {
        let (_order_id, price, amount) = order;

        let standardised_price = common::standardise_value(price);
        let standardised_amount = common::standardise_value(amount);

        if standardised_amount > 0 {
            bids.push((standardised_price, standardised_amount));
        } else {
            asks.push((standardised_price, standardised_amount.abs()));
        }
    }

    vec!(
        Broadcast::OrderbookSnapshot {
        source: Exchange::Bitfinex,
        pair, bids, asks
    })
}