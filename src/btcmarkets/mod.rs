use Broadcast;
use Exchange;

#[derive(Debug, Serialize)]
#[serde(untagged)]
enum Request {
    JoinQueue {
        #[serde(rename = "channelName")]
        channel_name: String,
        #[serde(rename = "eventName")]
        event_name: String
    }
}

pub fn connect(tx: ::ws::Sender, currency: &str, instrument: &str, use_proxy: bool) {

    let connect_addr = if use_proxy {
        "ws://localhost:10001"
    } else {
        "wss://socket.btcmarkets.net"
    };

    let requests = vec!(
        Request::JoinQueue {
            channel_name: format!("Orderbook_{}{}", instrument, currency),
            event_name: "OrderBookChange".to_string()
        },
        Request::JoinQueue {
            channel_name: format!("TRADE_{}{}", instrument, currency),
            event_name: "MarketTrade".to_string()
        }
    );

    ::ws::connect(connect_addr, |out| {
        println!("Successfully connected to BTCMarkets");

        requests.iter().for_each(|req| {
            match out.send(::serde_json::to_string(&req).unwrap()) {
                Ok(_) => println!("Sent {:?} to BtcMarkets", req),
                Err(e) => println!("Failed to send request to BtcMarkets: {}", e)
            }
        });

        // TODO: be less dumb with this temporary variable
        let tmp = tx.clone();
        move |msg: ::ws::Message| {

            println!("Raw message: {}", msg);

            if msg.is_text() {
                match msg.into_text() {
                    Ok(txt) => {
                        match ::serde_json::from_str::<Response>(&txt) {
                            Ok(response) => {
                                //TODO: map it
                                if let Some(mapped) = map(response) {
                                    let serialized = ::serde_json::to_string(&mapped).unwrap();
                                    if let Err(e) = tmp.send(serialized) {
                                        println!("Could not send broadcast: {}", e);
                                    }
                                }
                            },
                            Err(e) => println!("Could not deserialize BTCMarkets message {}: {}", txt, e)
                        };
                    },
                    Err(e) => println!("Could not convert text-based BTCMarkets message into text: {}", e)
                };
            } else {
                println!("Binary message");
            }

            Ok(())
        }
    }).unwrap();

}

fn map(response: Response) -> Option<Broadcast> {
    match response {
        Response::OrderbookChange { currency, instrument, bids, asks, .. } => Some(
            Broadcast::OrderbookUpdate {
                seq_num: 1,
                source: Exchange::BtcMarkets,
                pair: (currency, instrument),
                bids: bids.into_iter().map(|(price, amount, _)| (price, amount)).collect(),
                asks: asks.into_iter().map(|(price, amount, _)| (price, amount)).collect()
            }),




        Trade => {
            Some (
                Broadcast::Trade {
                    seq_num: 1,
                    source: Exchange::BtcMarkets
                }
            )
        }
        _ => None
    }
}


//Broadcast::OrderbookUpdate {
//seq_num: 1,
//source: Exchange::BtcMarkets,
//pair: (response.currency, response.instrument),
//bids: response.bids.map(|(price, amount, _)| (price, amount)).collect(),
//asks: response.asks.map(|(price, amount, _)| (price, amount)).collect()
//}

#[derive(Debug, Deserialize)]
#[serde(untagged)]
enum Response {
    OrderbookChange {
        currency: String,
        instrument: String,
        timestamp: i64,
        #[serde(rename = "marketId")]
        market_id: i64,
        #[serde(rename = "snapshotId")]
        snapshot_id: i64,
        bids: Vec<(i64, i64, i64)>, //price, amount, unknown (2001? pair code?)
        asks: Vec<(i64, i64, i64)> //price, amount, unknown
    },
    Trade {
        id: i64,
        timestamp: i64,
        #[serde(rename = "marketId")]
        market_id: i64,
        agency: String,
        instrument: String,
        currency: String,
        trades: Vec<(i64, i64, i64, i64)>
    },
    Status {
        status: String
    },
}

