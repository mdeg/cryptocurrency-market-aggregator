mod api;

use self::api::*;
use common::{self, Broadcast, BroadcastType, Exchange, ConnectionFactory, MarketHandler, CurrencyPair};
use handler;
use std::collections::HashMap;
use ws;

type ChannelsMap = HashMap<i32, CurrencyPair>;

pub struct BitfinexHandler {
    inner: handler::HandlerCore,
    channels: ChannelsMap,
    pairs: Vec<CurrencyPair>
}

impl ::ws::Handler for BitfinexHandler {
    fn on_open(&mut self, _: ws::Handshake) -> ws::Result<()> {
        info!("Connected to {}", Exchange::Bitfinex);

        let requests = Self::get_requests(&self.pairs);

        self.inner.send_upstream(requests);

        let open = Broadcast::ExchangeConnectionOpened {
            exchange: Exchange::Bitfinex,
            ts: common::timestamp()
        };

        self.inner.broadcast(BroadcastType::One(open));

        // TODO: review this
        Ok(())
    }

    fn on_message(&mut self, msg: ws::Message) -> ws::Result<()> {
        match msg.into_text() {
            Ok(txt) => {
                match ::serde_json::from_str::<Response>(&txt) {
                    Ok(response) => {
                        let message = handle_response(response, &mut self.channels);
                        self.inner.broadcast(message);
                    },
                    Err(e) => error!("Could not deserialize message: {}", e)
                }
            },
            Err(e) => error!("Could not convert message to text: {}", e)
        };

        //TODO: review this
        Ok(())
    }

    fn on_close(&mut self, _code: ws::CloseCode, reason: &str) {
        info!("Connection to {} has been lost: {}", Exchange::Bitfinex, reason);

        let closed = Broadcast::ExchangeConnectionClosed {
            exchange: Exchange::Bitfinex,
            ts: common::timestamp()
        };

        self.inner.broadcast(BroadcastType::One(closed));
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
    broadcast_tx: ws::Sender,
    pairs: Vec<CurrencyPair>
}

impl ConnectionFactory for BitfinexFactory {
    fn new(broadcast_tx: ws::Sender, pairs: Vec<CurrencyPair>) -> Self {
        Self { broadcast_tx, pairs }
    }

    fn get_connect_addr() -> ::url::Url {
        ::url::Url::parse(dotenv!("BITFINEX_ADDR")).unwrap()
    }
}

impl ws::Factory for BitfinexFactory {
    type Handler = BitfinexHandler;

    fn connection_made(&mut self, sender: ws::Sender) -> Self::Handler {
        Self::Handler {
            inner: handler::HandlerCore::new(self.broadcast_tx.clone(), sender),
            pairs: self.pairs.clone(),
            channels: HashMap::new()
        }
    }
}

fn handle_response(response: Response, channels: &mut ChannelsMap) -> BroadcastType {
    match response {
        // TODO: remove pair code duplication
        Response::OrderbookUpdate(channel_id, (_, price, amount)) => {
            let pair = channels.get(&channel_id).expect(
                &format!("Could not find channel ID {}", channel_id));
            map_orderbook_update(*pair, price, amount)
        },
        Response::Trade(channel_id, _trade_update_type, trades) => {
            let pair = channels.get(&channel_id).expect(
                &format!("Could not find channel ID {}", channel_id));
            map_trade(*pair, trades)
        },
        Response::SubscribeConfirmation { channel_id, symbol, .. } => {
            let pair_code = map_pair_code(&symbol);
            debug!("{} pair code {:?} maps to channel ID {}", Exchange::Bitfinex, pair_code, channel_id);
            channels.insert(channel_id, pair_code);
            BroadcastType::None
        },
        Response::InitialOrderbook(channel_id, orders) => {
            let pair = channels.get(&channel_id).expect(
                &format!("Could not find channel ID {}", channel_id));
            map_initial_orderbook(*pair, orders)
        },
        Response::InitialTrade(channel_id, trades) => {
            let pair = channels.get(&channel_id).expect(
                &format!("Could not find channel ID {}", channel_id));
            map_initial_trades(*pair, trades)
        }
        _ => BroadcastType::None
    }
}

// Pairs list: https://api.bitfinex.com/v1/symbols
fn map_pair_code(pair_code: &str) -> CurrencyPair {
    match pair_code {
        "tXRPBTC" => CurrencyPair::XRPBTC,
        _ => panic!("Could not map pair code {}", pair_code)
    }
}

fn map_orderbook_update(pair: CurrencyPair, price: f64, amount: f64) -> BroadcastType {
    let standardised_price = common::standardise_value(price);
    let standardised_amount = common::standardise_value(amount);

    let (mut bids, mut asks) = (vec!(), vec!());
    if standardised_amount > 0 {
        bids.push((standardised_price, standardised_amount));
    } else {
        asks.push((standardised_price, standardised_amount.abs()));
    }

    let broadcast = if standardised_price == 0 {
        Broadcast::OrderbookRemove {
            source: Exchange::Bitfinex,
            pair, bids, asks
        }
    } else {
        Broadcast::OrderbookUpdate {
            source: Exchange::Bitfinex,
            pair, bids, asks
        }
    };

    BroadcastType::One(broadcast)
}

fn map_trade(pair: CurrencyPair, trade: (OrderId, Timestamp, Amount, Price)) -> BroadcastType {
    let (_order_id, ts, amount, price) = trade;
    let standardised_price = common::standardise_value(price);
    let standardised_amount = common::standardise_value(amount);

    let broadcast = Broadcast::Trade {
        source: Exchange::Bitfinex,
        pair,
        trade: (ts as i64, standardised_price, standardised_amount, standardised_price * standardised_amount)
    };

    BroadcastType::One(broadcast)
}

fn map_initial_trades(pair: CurrencyPair, trades: Vec<(OrderId, Timestamp, Amount, Price)>) -> BroadcastType {
    let trades_out = trades.into_iter().map(|(_order_id, ts, amount, price)| {
        let standardised_price = common::standardise_value(price);
        let standardised_amount = common::standardise_value(amount);

        (ts as i64, standardised_price, standardised_amount, standardised_price * standardised_amount)
    }).collect();

    let broadcast = Broadcast::TradeSnapshot {
        source: Exchange::Bitfinex,
        pair,
        trades: trades_out
    };

    BroadcastType::One(broadcast)
}

fn map_initial_orderbook(pair: CurrencyPair, orders: Vec<(OrderId, Price, Amount)>) -> BroadcastType {
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

    let broadcast = Broadcast::OrderbookSnapshot {
        source: Exchange::Bitfinex,
        pair, bids, asks
    };

    BroadcastType::One(broadcast)
}