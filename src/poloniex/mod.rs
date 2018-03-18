use common::{Broadcast, Exchange, MarketRunner};

struct PoloniexMarketRunner;

impl MarketRunner<Request, Response> for PoloniexMarketRunner {
    fn get_connect_addr() -> &'static str {
        "wss://api2.poloniex.com"
    }

    fn connect(&self, tx: ::ws::Sender, pairs: Vec<(String, String)>) {
        let requests = Self::get_requests(pairs);

        ::ws::connect(Self::get_connect_addr(), |out| {
            info!("[POLONIEX] Connected");

            requests.iter().for_each(|req| {
                match out.send(::serde_json::to_string(&req).unwrap()) {
                    Ok(_) => info!("[POLONIEX] Sent {:?}", req),
                    Err(e) => error!("[POLONIEX] Failed to send request: {}", e)
                }
            });

            let tx_in = tx.clone();
            move |msg: ::ws::Message| {
                debug!("[POLONIEX] Raw message: {}", msg);

                match msg.into_text() {
                    Ok(txt) => {
                        match ::serde_json::from_str::<Response>(&txt) {
                            Ok(response) => {
                                if let Some(mapped) = self.map(response) {
                                    let serialized = ::serde_json::to_string(&mapped).unwrap();
                                    tx_in.send(serialized)
                                        .unwrap_or_else(|e| error!("[POLONIEX] Could not broadcast: {}", e));
                                }
                            },
                            Err(e) => error!("[POLONIEX] Could not deserialize message: {}", e)
                        }
                    },
                    Err(e) => error!("[POLONIEX] Could not convert message to text: {}", e)
                }

                Ok(())
            }
        }).unwrap();
    }

    fn get_requests(pairs: Vec<(String, String)>) -> Vec<Request> {
        pairs.into_iter().map(|(ref first, ref second)| {
            Request::JoinQueue {
                command: "subscribe".to_string(),
                channel: format!("{}_{}", first, second)
            }
        }).collect()
    }

    fn map(&self, response: Response) -> Option<Broadcast> {
        unimplemented!()
    }
}

#[derive(Debug, Deserialize)]
#[serde(untagged)]
enum Response {
    // TODO: custom deserialization for this
    // currency_pair_id, sequence_number,
    OrderBook(i32, i64, ((String /*value is always "i"*/, InternalOrderStructure))),
    Error {
        #[serde(rename = "error")]
        reason: String
    },
    Heartbeat([i32; 1]),
    HeartbeatFromHeartbeatChannel((i32, i64))
}

#[derive(Debug, Deserialize)]
struct InternalOrderStructure {
    #[serde(rename = "currencyPair")]
    currency_pair_string: String,
    #[serde(rename = "orderBook")]
    order_book: (Vec<(i64, i64)>, Vec<(i64, i64)>)
}

#[derive(Debug, Serialize)]
#[serde(untagged)]
enum Request {
    JoinQueue {
        command: String,
        channel: String
    }
}

// Responses for API v1 (defunct?)
//#[derive(Debug, Deserialize)]
//#[serde(tag = "type")]
//enum PoloniexV1Response {
//    #[serde(rename = "orderBookModify")]
//    OrderBookModify {
//        rate: f64,
//        #[serde(rename = "type")]
//        order_type: String,
//        amount: f64
//    },
//    #[serde(rename = "orderBookRemove")]
//    OrderBookRemove {
//        rate: f64,
//        #[serde(rename = "type")]
//        order_type: String
//    },
//    #[serde(rename = "newTrade")]
//    Trade {
//        #[serde(rename = "tradeId")]
//        trade_id: String,
//        rate: f64,
//        amount: f64,
//        date: String,
//        total: f64,
//        #[serde(rename = "type")]
//        trade_type: String
//    }
//}