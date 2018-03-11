use Broadcast;
use Exchange;

#[derive(Debug, Serialize)]
#[serde(untagged)]
enum PoloniexRequest<T> {
    JoinQueue {
        command: String,
        channel: T
    }
}

fn connect(tx: ::ws::Sender, currency: &str, instrument: &str) {
    let connect_addr = "wss://api2.poloniex.com";

    let requests = vec!(
        PoloniexRequest::JoinQueue::<String> {
            command: "subscribe".to_string(),
            //            channel: format!("{}_{}", currency, instrument)
            channel: format!("{}_{}", "BTC", "XMR")
        },
//        PoloniexRequest::JoinQueue::<i32> {
//            command: "subscribe".to_string(),
//            channel: 1010
//        }
    );

    ::ws::connect(connect_addr, |out| {
        println!("Successfully connected to Poloniex");

        requests.iter().for_each(|req| {
            match out.send(::serde_json::to_string(&req).unwrap()) {
                Ok(_) => println!("Sent {:?} to Poloniex", req),
                Err(e) => println!("Failed to send request to Poloniex: {}", e)
            }
        });

        let tmp = tx.clone();
        move |msg: ::ws::Message| {

            println!("Raw message: {}", msg);

            if msg.is_text() {
                match msg.into_text() {
                    Ok(txt) => {
                        // TODO: do this better
                        if !txt.contains("type") {
                            // untagged
                            match ::serde_json::from_str::<UntaggedPoloniexResponse>(&txt) {
                                Ok(response) => {
                                    match response {
                                        UntaggedPoloniexResponse::Error { reason } => {
                                            println!("Received error from Poloniex! {}", reason);
                                        },
                                        _ => {}
                                    }
                                },
                                Err(e) => panic!("Could not deserialize Poloniex message {}: {}", txt, e)
                            };
                        } else {
                            match ::serde_json::from_str::<PoloniexResponse>(&txt) {
                                Ok(response) => {
                                    let serialized =
                                        ::serde_json::to_string(&Broadcast::Heartbeat {}).unwrap();
                                    if let Err(e) = tmp.send(serialized) {
                                        println!("Could not send passthrough message: {}", e);
                                    }
                                },
                                Err(e) => panic!("Could not deserialize Poloniex message {}: {}", txt, e)
                            };
                        }
                    },
                    Err(e) => println!("Could not convert text-based Poloniex message into text: {}", e)
                };
            } else {
                println!("Binary message");
            }

            Ok(())
        }
    });
}

#[derive(Debug, Deserialize)]
#[serde(untagged)]
enum PoloniexResponse {
    OrderBook (
        i32, //currency pair ID
        i64, //sequence number
        ((
            String, //value is "i". wtf?
            TmpInternal
        ))
    )
}

#[derive(Debug, Deserialize)]
struct TmpInternal {
    #[serde(rename = "currencyPair")]
    currency_pair_name: String,
    #[serde(rename = "orderBook")]
    order_book: (Vec<(i64, i64)>, Vec<(i64, i64)>)
//    order_book: (Vec<x:y>, Vec<x:y>)
}


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

#[derive(Debug, Deserialize)]
#[serde(untagged)]
enum UntaggedPoloniexResponse {
    Error {
        #[serde(rename = "error")]
        reason: String
    },
    Heartbeat([i32; 1]),
    HeartbeatFromHeartbeatChannel((i32, i64))
}