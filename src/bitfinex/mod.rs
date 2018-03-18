use common::{Broadcast, Exchange};
use common::MarketRunner;

pub struct BitfinexMarketRunner;

impl MarketRunner<Request, Response> for BitfinexMarketRunner {
    fn get_connect_addr() -> &'static str {
        "wss://api.bitfinex.com/ws/2"
    }

    fn connect(&mut self, tx: ::ws::Sender, pairs: Vec<(String, String)>) {
        let requests = Self::get_requests(pairs);

        ::ws::connect(Self::get_connect_addr(), move |out| {
            info!("Successfully connected to Bitfinex");

            requests.iter().for_each(|req| {
                match out.send(::serde_json::to_string(&req).unwrap()) {
                    Ok(_) => info!("Sent {:?} to Bitfinex", req),
                    Err(e) => error!("Failed to send request to BtcMarkets: {}", e)
                }
            });

            // Need to clone this to send into FnMut closure
            let tx_in = tx.clone();
            move |msg: ::ws::Message| {
                debug!("Raw message from Bitfinex: {}", msg);

                match msg.into_text() {
                    Ok(txt) => {
                        match ::serde_json::from_str::<Response>(&txt) {
                            Ok(response) => {
                                if let Some(mapped) = self.map(response) {
                                    let serialized = ::serde_json::to_string(&mapped).unwrap();
                                    tx_in.send(serialized).unwrap_or_else(|e| error!("Could not send broadcast: {}", e));
                                }
                            }
                            Err(e) => error!("Could not deserialize Bitfinex message: {}", e)
                        }
                    },
                    Err(e) => error!("Could not convert Bitfinex message to text: {}", e)
                };

                Ok(())
            }
        }).unwrap();
    }

    fn get_requests(pairs: Vec<(String, String)>) -> Vec<Request> {
        pairs.into_iter().map(|(ref first, ref second)| {
            Request::JoinQueue {
                event: "subscribe".to_string(),
                channel: "book".to_string(),
                //TODO: proper currency codes (order is not normalisable)
                symbol: format!("t{}{}", second, first),
                precision: Precision::R0,
                frequency: Frequency::F0,
                length: 100.to_string()
            }
        }).collect()
    }

    fn map(&mut self, response: Response) -> Option<Broadcast> {
        match response {
            Response::OrderbookUpdate(symbol, (_, price, amount)) => {
                Some(self.map_orderbook_update(symbol, price, amount))
            },
            _ => None
        }
    }
}

impl BitfinexMarketRunner {

    fn map_orderbook_update(&self, symbol: i32, price: f64, amount: f64) -> Broadcast {

        //TODO: order deletion messages
//    if price == 0 {
//        // delete order
//    } else {
//
//    }

        let conv_price = (price * ::MULTIPLIER as f64) as i64;
        let conv_amount = (amount * ::MULTIPLIER as f64) as i64;

        let (mut bids, mut asks) = (vec!(), vec!());
        //TODO: trading vs funding?
        if conv_amount > 0 {
            bids.push((conv_price, conv_amount));
        } else {
            asks.push((conv_price, conv_amount.abs()));
        }

        Broadcast::OrderbookUpdate {
            seq_num: 1,
            source: Exchange::Bitfinex,
            // TODO: work out channels for this
            pair: ("XRP".to_string(), "BTC".to_string()),
            bids,
            asks
        }
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

