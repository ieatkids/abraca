use abraca_macros::clike_enum;
use chrono::NaiveDate;
use serde::{Deserialize, Serialize};
use std::str::FromStr;
use strum_macros::{Display, EnumString};

pub type Result<T> = anyhow::Result<T>;

clike_enum!(Ccy, "fixtures/ccys.txt");

#[derive(Debug, Copy, Clone, PartialEq, Deserialize, Serialize)]
pub enum Side {
    Buy,
    Sell,
}

#[derive(Debug, Copy, Clone, PartialEq, Deserialize, Serialize)]
pub enum OrdType {
    Market,
    Limit,
    PostOnly,
    Fok,
    Ioc,
}

#[derive(Debug, Copy, Clone, PartialEq, Deserialize, Serialize)]
pub enum TdMode {
    Isolated,
    Cross,
    Cash,
}

#[derive(Debug, Copy, Clone, PartialEq, Deserialize, Serialize)]
pub enum MgnMode {
    Isolated,
    Cross,
    Cash,
}

#[derive(Debug, Copy, Clone, PartialEq)]
pub enum ExecType {
    Taker,
    Maker,
}

#[derive(Debug, Default, Copy, Clone, PartialEq, Serialize, Deserialize)]
pub enum OrdState {
    #[default]
    Unknwon,
    Live,
    PartiallyFilled,
    Filled,
    Canceled,
    Rejected,
}

#[derive(Debug, Copy, Clone, PartialEq, Serialize, Deserialize)]
pub enum CtType {
    Linear,
    Inverse,
}

#[derive(Debug, Copy, Clone, PartialEq, Hash, Serialize, Deserialize, EnumString, Display)]
pub enum OptType {
    C,
    P,
}

#[non_exhaustive]
#[derive(Debug, Copy, Clone, Hash, Eq, PartialEq, Serialize, Deserialize, EnumString, Display)]
pub enum Exch {
    Okx,
    BinanceFutures,
}

#[derive(Debug, Copy, Clone, PartialEq, Hash, Serialize, Deserialize, EnumString, Display)]
pub enum DataType {
    Ticker,
    FundingRate,
    OpenInterest,
    Depth,
    Trade,
}

#[derive(Debug, Copy, Clone, PartialEq, Hash, Serialize, Deserialize)]
pub enum InstType {
    Spot,
    Margin,
    Swap,
    Futures(NaiveDate),
    Options(NaiveDate, i64, OptType),
}

impl ToString for InstType {
    fn to_string(&self) -> String {
        match self {
            InstType::Spot => "Spot".to_string(),
            InstType::Margin => "Margin".to_string(),
            InstType::Swap => "Swap".to_string(),
            InstType::Futures(d) => format!("Futures-{}", d.format("%y%m%d")),
            InstType::Options(d, stk, t) => {
                format!("Options-{}-{}-{}", d.format("%y%m%d"), stk, t.to_string())
            }
        }
    }
}

impl TryFrom<&str> for InstType {
    type Error = String;

    fn try_from(value: &str) -> std::result::Result<Self, Self::Error> {
        let mut parts = value.split('-');
        let Some(p0) = parts.next() else{
            return Err(anyhow::anyhow!("empty string").to_string());
        };
        match p0 {
            "Spot" => Ok(InstType::Spot),
            "Margin" => Ok(InstType::Margin),
            "Swap" => Ok(InstType::Swap),
            "Futures" => {
                let Some(p1) = parts.next() else{
                    return Err(anyhow::anyhow!("expiration date not provided").to_string());
                };
                let exp_date = chrono::NaiveDate::parse_from_str(p1, "%y%m%d")
                    .map_err(|_| "invalid expiration date")?;
                Ok(InstType::Futures(exp_date))
            }
            "Options" => {
                let Some(p1) = parts.next() else{
                    return Err(anyhow::anyhow!("expiration date not provided").to_string());
                };
                let Some(p2) = parts.next() else{
                    return Err(anyhow::anyhow!("strike not provided").to_string());
                };
                let Some(p3) = parts.next() else{
                    return Err(anyhow::anyhow!("option type not provided").to_string());
                };
                let exp_date = chrono::NaiveDate::parse_from_str(p1, "%y%m%d")
                    .map_err(|_| "invalid expiration date")?;
                let strike = p2.parse::<i64>().map_err(|_| "invalid strike")?;
                let opt_type = OptType::from_str(p3).map_err(|_| "invalid option type")?;
                Ok(InstType::Options(exp_date, strike, opt_type))
            }
            _ => Err(anyhow::anyhow!("invalid instrument type").to_string()),
        }
    }
}

#[derive(Debug, Clone, Hash, PartialEq, Serialize, Deserialize)]
pub struct Inst {
    pub exch: Exch,
    pub base_ccy: Ccy,
    pub quote_ccy: Ccy,
    pub inst_type: InstType,
}

impl ToString for Inst {
    fn to_string(&self) -> String {
        format!(
            "{}.{}.{}.{}",
            self.exch,
            self.base_ccy,
            self.quote_ccy,
            self.inst_type.to_string()
        )
    }
}

impl TryFrom<&str> for Inst {
    type Error = String;

    fn try_from(value: &str) -> std::result::Result<Self, Self::Error> {
        let parts: Vec<_> = value.split('.').collect();
        if parts.len() != 4 {
            return Err("invalid instrument".to_string());
        }
        let Ok(exch) = Exch::try_from(parts[0]) else{
            return Err("invalid exchange".to_string());
        };
        let Ok(base_ccy) = Ccy::try_from(parts[1]) else{
            return Err("invalid base currency".to_string());
        };
        let Ok(quote_ccy) = Ccy::try_from(parts[2]) else{
            return Err("invalid quote currency".to_string());
        };
        let Ok(inst_type) = InstType::try_from(parts[3]) else{
            return Err("invalid instrument type".to_string());
        };
        Ok(Inst {
            exch,
            base_ccy,
            quote_ccy,
            inst_type,
        })
    }
}

impl<E, C, I> TryFrom<(E, C, C, I)> for Inst
where
    E: TryInto<Exch>,
    C: TryInto<Ccy>,
    I: TryInto<InstType>,
{
    type Error = String;

    fn try_from(
        (exch, base_ccy, quote_ccy, inst_type): (E, C, C, I),
    ) -> std::result::Result<Self, Self::Error> {
        let Ok(exch) = exch.try_into() else{
            return Err("invalid exchange".to_string());
        };
        let  Ok(base_ccy) = base_ccy.try_into() else{
            return Err("invalid base currency".to_string());
        };
        let  Ok(quote_ccy) = quote_ccy.try_into() else{
            return Err("invalid quote currency".to_string());
        };
        let  Ok(inst_type) = inst_type.try_into() else{
            return Err("invalid instrument type".to_string());
        };
        Ok(Inst {
            exch,
            base_ccy,
            quote_ccy,
            inst_type,
        })
    }
}

#[cfg(test)]
mod tests {

    use super::*;

    #[test]
    fn foo() {
        let path = std::path::PathBuf::from(std::env::var("CARGO_MANIFEST_DIR").unwrap());
        println!("{:?}", path.parent().unwrap());
    }

    #[test]
    fn try_into_inst_type_works() {
        assert_eq!("Spot".try_into(), Ok(InstType::Spot));
        assert_eq!("Margin".try_into(), Ok(InstType::Margin));
        assert_eq!("Swap".try_into(), Ok(InstType::Swap));
        assert_eq!(
            "Futures-230421".try_into(),
            Ok(InstType::Futures(
                NaiveDate::from_ymd_opt(2023, 4, 21).unwrap()
            ))
        );
        assert_eq!(
            "Options-230421-10000-C".try_into(),
            Ok(InstType::Options(
                NaiveDate::from_ymd_opt(2023, 4, 21).unwrap(),
                10000,
                OptType::C
            ))
        );
    }

    #[test]
    fn try_into_inst_works() {
        assert_eq!(
            ("Okx", "BTC", "USDT", "Spot").try_into(),
            Ok(Inst {
                exch: Exch::Okx,
                base_ccy: Ccy::BTC,
                quote_ccy: Ccy::USDT,
                inst_type: InstType::Spot,
            })
        );
        assert_eq!(
            ("Okx", "ETH", "USD", "Swap").try_into(),
            Ok(Inst {
                exch: Exch::Okx,
                base_ccy: Ccy::ETH,
                quote_ccy: Ccy::USD,
                inst_type: InstType::Swap,
            })
        );
        assert_eq!(
            ("Okx", "BTC", "USDT", "Futures-230421").try_into(),
            Ok(Inst {
                exch: Exch::Okx,
                base_ccy: Ccy::BTC,
                quote_ccy: Ccy::USDT,
                inst_type: InstType::Futures(NaiveDate::from_ymd_opt(2023, 4, 21).unwrap()),
            })
        );
        assert_eq!(
            ("Okx", "BTC", "USDT", "Options-230421-10000-C").try_into(),
            Ok(Inst {
                exch: Exch::Okx,
                base_ccy: Ccy::BTC,
                quote_ccy: Ccy::USDT,
                inst_type: InstType::Options(
                    NaiveDate::from_ymd_opt(2023, 4, 21).unwrap(),
                    10000,
                    OptType::C
                ),
            })
        );
    }
}
