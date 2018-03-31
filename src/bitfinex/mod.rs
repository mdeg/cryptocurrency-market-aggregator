mod api;

use common::{Broadcast, Exchange, ConnectionFactory, MarketHandler, CurrencyPair};

pub struct BitfinexHandler {
    broadcast_tx: ::ws::Sender,
    pairs: Vec<CurrencyPair>,
    out_tx: ::ws::Sender
}

impl ::ws::Handler for BitfinexHandler {
    fn on_open(&mut self, _: ::ws::Handshake) -> ::ws::Result<()> {
        info!("[BITFINEX] Connected");

        Self::get_requests(&self.pairs).into_iter().for_each(|req| {
            self.out_tx.send(req)
                .unwrap_or_else(|e| error!("[BITFINEX] Failed to send request: {}", e));
        });

        Ok(())
    }

    fn on_message(&mut self, msg: ::ws::Message) -> ::ws::Result<()> {
        debug!("[BITFINEX] Raw message: {}", msg);

        match msg.into_text() {
            Ok(txt) => {
                match ::serde_json::from_str::<api::Response>(&txt) {
                    Ok(response) => {
                        Mapper::map(response).into_iter()
                            .map(|r| ::serde_json::to_string(&r).unwrap())
                            .for_each(|msg| self.broadcast_tx.send(msg)
                                .unwrap_or_else(|e| error!("[BITFINEX] Could not broadcast: {}", e)))
                    },
                    Err(e) => error!("[BITFINEX] Could not deserialize message: {}", e)
                }
            },
            Err(e) => error!("[BITFINEX] Could not convert Bitfinex message to text: {}", e)
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
            CurrencyPair::BTCXRP => "tXRPBTC"
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
        ::url::Url::parse("wss://api.bitfinex.com/ws/2").unwrap()
    }
}

impl ::ws::Factory for BitfinexFactory {
    type Handler = BitfinexHandler;

    fn connection_made(&mut self, sender: ::ws::Sender) -> Self::Handler {
        Self::Handler {
            broadcast_tx: self.broadcast_tx.clone(),
            pairs: self.pairs.clone(),
            out_tx: sender
        }
    }
}

struct Mapper;

impl Mapper {

    fn map(response: api::Response) -> Vec<Broadcast> {
        match response {
            api::Response::OrderbookUpdate(channel_id, (_, price, amount)) => {
                // TODO: map pair from channel_id
                let pair = CurrencyPair::BTCXRP;
                Self::map_orderbook_update(pair, price, amount)
            },
            api::Response::Trade(channel_id, unknown_string, trades) => {
                let pair = CurrencyPair::BTCXRP;
                Self::map_trade(pair, trades)
            }
            // TODO: map initial responses
            _ => vec!()
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
}