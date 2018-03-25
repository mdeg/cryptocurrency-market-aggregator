use common::{Broadcast, Exchange, MarketRunner, CurrencyPair};

pub struct BitfinexMarketRunner {
    broadcast_tx: ::ws::Sender,
    pairs: Vec<CurrencyPair>,
    out_tx: ::ws::Sender
}

impl ::ws::Handler for BitfinexMarketRunner {
    fn on_open(&mut self, _: ::ws::Handshake) -> ::ws::Result<()> {
        info!("[BITFINEX] Connected");

        Self::get_requests(&self.pairs).iter().for_each(|req| {
            match self.out_tx.send(::serde_json::to_string(&req).unwrap()) {
                Ok(_) => info!("[BITFINEX] Sent {:?}", req),
                Err(e) => error!("[BITFINEX] Failed to send request: {}", e)
            }
        });

        Ok(())
    }

    fn on_message(&mut self, msg: ::ws::Message) -> ::ws::Result<()> {
        debug!("[BITFINEX] Raw message: {}", msg);

        match msg.into_text() {
            Ok(txt) => {
                match ::serde_json::from_str::<Response>(&txt) {
                    Ok(response) => {
                        self.map(response).into_iter()
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

impl MarketRunner<Request, Response> for BitfinexMarketRunner {
    fn connect(broadcast_tx: ::ws::Sender, pairs: Vec<CurrencyPair>) {
        let factory = BitfinexRunnerWsFactory { broadcast_tx, pairs };
        let mut ws = ::ws::Builder::new().build(factory).unwrap();
        ws.connect(Self::get_connect_addr()).unwrap();
        ws.run().unwrap();
    }

    fn map(&mut self, response: Response) -> Vec<Broadcast> {
        match response {
            Response::OrderbookUpdate(channel_id, (_, price, amount)) => {
                // TODO: map pair from channel_id
                let pair = CurrencyPair::BTCXRP;
                self.map_orderbook_update(pair, price, amount)
            },
            Response::Trade(channel_id, trades) => {
                let pair = CurrencyPair::BTCXRP;
                self.map_trade(pair, trades)
            }
            // TODO: map initials
            _ => vec!()
        }
    }

    fn get_connect_addr() -> ::url::Url {
        ::url::Url::parse("wss://api.bitfinex.com/ws/2").unwrap()
    }

    fn get_requests(pairs: &[CurrencyPair]) -> Vec<Request> {
        pairs.iter().flat_map(|ref pair| { vec!(
//            Request::JoinQueue {
//                event: "subscribe".to_string(),
//                channel: "book".to_string(),
//                symbol: Self::stringify_pair(*pair),
//                precision: Precision::R0,
//                frequency: Frequency::F0,
//                length: 100.to_string()
//            },
            Request::JoinQueue {
                event: "subscribe".to_string(),
                channel: "trades".to_string(),
                symbol: Self::stringify_pair(*pair),
                precision: Precision::R0,
                frequency: Frequency::F0,
                length: 100.to_string()
            }
        )}).collect()
    }

    fn stringify_pair(pair: &CurrencyPair) -> String {
        match *pair {
            CurrencyPair::BTCXRP => "tXRPBTC"
        }.to_string()
    }
}

struct BitfinexRunnerWsFactory {
    broadcast_tx: ::ws::Sender,
    pairs: Vec<CurrencyPair>
}

impl ::ws::Factory for BitfinexRunnerWsFactory {
    type Handler = BitfinexMarketRunner;

    fn connection_made(&mut self, sender: ::ws::Sender) -> Self::Handler {
        BitfinexMarketRunner {
            broadcast_tx: self.broadcast_tx.clone(),
            pairs: self.pairs.clone(),
            out_tx: sender
        }
    }
}

impl BitfinexMarketRunner {

    fn map_orderbook_update(&self, pair: CurrencyPair, price: f64, amount: f64) -> Vec<Broadcast> {
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

    fn map_trade(&self, pair: CurrencyPair, trades: Vec<(i64, f64, f64, f64)>) -> Vec<Broadcast> {
        let trades_out = trades.into_iter().map(|(order_id, ts, amount, price)| {
            let conv_price = (price * ::MULTIPLIER as f64) as i64;
            let conv_amount = (amount * ::MULTIPLIER as f64) as i64;

            (ts as i64, conv_price, conv_amount, conv_price * conv_amount)
        }).collect();

        vec!(Broadcast::Trade {
            source: Exchange::Bitfinex,
            pair,
            trades: trades_out
        })
    }
}

#[derive(Debug, Deserialize)]
#[serde(untagged)]
pub enum Response {
    Connected {
        event: String,
        version: i32,
        platform: ConnectedResponsePlatform
    },
    SubscribeConfirmation {
        event: String,
        channel: String,
        #[serde(rename = "chanId")]
        channel_id: i32,
        pair: String,
        symbol: String,
        #[serde(rename = "prec")]
        precision: Precision,
        #[serde(rename = "freq")]
        frequency: Frequency,
        #[serde(rename = "len")]
        length: String,
    },
    SubscribeError {
        event: String,
        channel: String,
        code: i32,
        msg: String,
        pair: String,
        symbol: String,
        #[serde(rename = "prec")]
        precision: Precision,
        #[serde(rename = "freq")]
        frequency: Frequency,
        #[serde(rename = "len")]
        length: String,
    },
    Heartbeat(i32, String), //channelid, string="hb"
    // TODO: handle initials
    Trade(i32, Vec<(i64, f64, f64, f64)>), //channelId, (orderId, ts, amount, price)
    InitialOrderbook(i32, Vec<(i64, f64, f64)>), //channelId, (orderid, price, amount)
    OrderbookUpdate(i32, (i64, f64, f64)) //channelId, (orderid, price, amount)
}

// TODO: work out a way to do this inline in the enum
#[derive(Debug, Deserialize)]
pub struct ConnectedResponsePlatform {
    status: i32
}

#[derive(Debug, Serialize)]
#[serde(untagged)]
pub enum Request {
    JoinQueue {
        event: String,
        channel: String,
        symbol: String,
        #[serde(rename = "prec")]
        precision: Precision,
        #[serde(rename = "freq")]
        frequency: Frequency,
        #[serde(rename = "len")]
        length: String
    }
}

#[derive(Debug, Deserialize, Serialize)]
pub enum Precision {
    R0,
    P0,
    P1,
    P2,
    P3
}

#[derive(Debug, Deserialize, Serialize)]
pub enum Frequency {
    F0,
    F1
}

