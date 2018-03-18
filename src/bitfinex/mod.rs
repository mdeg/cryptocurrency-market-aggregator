use common::{Broadcast, Exchange};
use common::MarketRunner;

pub struct BitfinexMarketRunner {
    broadcast_tx: ::ws::Sender,
    pairs: Vec<(String, String)>,
    out_tx: ::ws::Sender,
    // TODO: increment this
    seq_num: i64
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
                        if let Some(mapped) = self.map(response) {
                            let serialized = ::serde_json::to_string(&mapped).unwrap();
                            self.broadcast_tx.send(serialized).unwrap_or_else(|e| error!("[BITFINEX] Could not send broadcast: {}", e));
                        }
                    }
                    Err(e) => error!("[BITFINEX] Could not deserialize message: {}", e)
                }
            },
            Err(e) => error!("[BITFINEX] Could not convert Bitfinex message to text: {}", e)
        };

        Ok(())
    }
}

impl MarketRunner<Request, Response> for BitfinexMarketRunner {
    fn connect(broadcast_tx: ::ws::Sender, pairs: Vec<(String, String)>) {
        let factory = BitfinexRunnerWsFactory { broadcast_tx, pairs };
        let mut ws = ::ws::Builder::new().build(factory).unwrap();
        ws.connect(Self::get_connect_addr()).unwrap();
        ws.run().unwrap();
    }

    fn map(&mut self, response: Response) -> Option<Broadcast> {
        match response {
            Response::OrderbookUpdate(symbol, (_, price, amount)) => {
                Some(self.map_orderbook_update(symbol, price, amount))
            },
            _ => None
        }
    }

    fn get_connect_addr() -> ::url::Url {
        ::url::Url::parse("wss://api.bitfinex.com/ws/2").unwrap()
    }

    fn get_requests(pairs: &[(String, String)]) -> Vec<Request> {
        pairs.iter().map(|&(ref first, ref second)| {
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
}

struct BitfinexRunnerWsFactory {
    broadcast_tx: ::ws::Sender,
    pairs: Vec<(String, String)>
}

impl ::ws::Factory for BitfinexRunnerWsFactory {
    type Handler = BitfinexMarketRunner;

    fn connection_made(&mut self, sender: ::ws::Sender) -> Self::Handler {
        BitfinexMarketRunner {
            broadcast_tx: self.broadcast_tx.clone(),
            pairs: self.pairs.clone(),
            out_tx: sender,
            seq_num: 0
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

