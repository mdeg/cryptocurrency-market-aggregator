use std::fmt;

#[derive(Debug, Serialize, Copy, Clone, PartialEq, Ord, Eq, PartialOrd)]
pub enum CurrencyPair {
    XRPBTC
}

impl CurrencyPair {
    pub fn parse(values: &str) -> Vec<CurrencyPair> {
        let mut pairs: Vec<CurrencyPair> = values.split(',')
            .map(|x| Self::map(x).expect(&format!("Could not parse currency pair {}", x)))
            .collect();
        pairs.sort_unstable();
        pairs.dedup();
        pairs
    }

    pub fn map(value: &str) -> Option<CurrencyPair> {
        match value {
            "BTCXRP" => Some(CurrencyPair::XRPBTC),
            _ => None
        }
    }
}

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