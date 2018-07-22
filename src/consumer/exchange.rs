use std::fmt;

#[derive(Debug, Serialize)]
#[serde(rename_all = "lowercase")]
pub enum Exchange {
    BtcMarkets,
    Bitfinex
}

impl fmt::Display for Exchange {
    fn fmt(&self, f: &mut fmt::Formatter) -> Result<(), fmt::Error> {
        match self {
            Exchange::BtcMarkets => write!(f, "BTCMarkets"),
            Exchange::Bitfinex => write!(f, "Bitfinex")
        }
    }
}