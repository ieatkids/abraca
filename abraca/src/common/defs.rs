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
#[derive(Debug, Copy, Clone, Hash, Eq, PartialEq, Serialize, Deserialize, EnumString)]
pub enum Exch {
    Okx,
    BinanceFutures,
}

#[derive(Debug, Copy, Clone, PartialEq, Hash, Serialize, Deserialize)]
pub enum InstType {
    Spot,
    Margin,
    Swap,
    Futures(NaiveDate),
    Options(NaiveDate, i64, OptType),
}

impl TryFrom<&str> for InstType {
    type Error = String;

    fn try_from(value: &str) -> std::result::Result<Self, Self::Error> {
        let parts: Vec<_> = value.split('-').collect();
        let inst_type = match parts[0] {
            "Spot" => InstType::Spot,
            "Margin" => InstType::Margin,
            "Swap" => InstType::Swap,
            "Futures" => {
                let exp_date = chrono::NaiveDate::parse_from_str(parts[1], "%y%m%d")
                    .map_err(|_| "invalid expiration date")?;
                InstType::Futures(exp_date)
            }
            "Options" => {
                let exp_date = chrono::NaiveDate::parse_from_str(parts[1], "%y%m%d")
                    .map_err(|_| "invalid expiration date")?;
                let strike = parts[2].parse::<i64>().map_err(|_| "invalid strike")?;
                let opt_type = OptType::from_str(parts[3]).map_err(|_| "invalid option type")?;
                InstType::Options(exp_date, strike, opt_type)
            }
            _ => return Err("invalid instrument type".to_string()),
        };
        Ok(inst_type)
    }
}

#[derive(Debug, Clone, Hash, PartialEq, Serialize, Deserialize)]
pub struct Inst {
    pub exch: Exch,
    pub base_ccy: Ccy,
    pub quote_ccy: Ccy,
    pub inst_type: InstType,
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
