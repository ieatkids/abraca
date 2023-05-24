use chrono::NaiveDateTime;

use crate::prelude::*;

pub(super) fn inst_to_str(inst: &Inst) -> String {
    match inst.inst_type {
        InstType::Spot => format!("{}-{}", inst.base_ccy, inst.quote_ccy),
        InstType::Margin => format!("{}-{}", inst.base_ccy, inst.quote_ccy),
        InstType::Swap => format!("{}-{}-SWAP", inst.base_ccy, inst.quote_ccy),
        InstType::Futures(exp_date) => {
            format!(
                "{}-{}-{}",
                inst.base_ccy,
                inst.quote_ccy,
                exp_date.format("%y%m%d")
            )
        }
        InstType::Options(exp_date, stk, opt_type) => {
            format!(
                "{}-{}-{}-{}-{}",
                inst.base_ccy,
                inst.quote_ccy,
                exp_date.format("%y%m%d"),
                stk,
                opt_type
            )
        }
    }
}

pub(super) fn inst_type_to_str(inst_type: &InstType) -> &'static str {
    match inst_type {
        InstType::Spot => "SPOT",
        InstType::Margin => "MARGIN",
        InstType::Swap => "SWAP",
        InstType::Futures(_) => "FUTURES",
        InstType::Options(_, _, _) => "OPTION",
    }
}
pub(super) fn side_to_str(s: &Side) -> &'static str {
    match s {
        Side::Buy => "buy",
        _ => "sell",
    }
}

pub(super) fn ord_type_to_str(ot: &OrdType) -> &'static str {
    match ot {
        OrdType::Market => "market",
        OrdType::Limit => "limit",
        OrdType::PostOnly => "post_only",
        OrdType::Fok => "fok",
        OrdType::Ioc => "ioc",
    }
}

pub(super) fn td_mod_to_str(tm: &TdMode) -> &'static str {
    match tm {
        TdMode::Isolated => "isolated",
        TdMode::Cross => "cross",
        TdMode::Cash => "cash",
    }
}

pub(super) fn str_to_inst(s: &str) -> Inst {
    let parts: Vec<&str> = s.split('-').collect();
    let base_ccy: Ccy = parts[0].try_into().unwrap();
    let quote_ccy: Ccy = parts[1].try_into().unwrap();
    let inst_type = match parts.len() {
        2 => InstType::Spot,
        3 => {
            if parts[2] == "SWAP" {
                InstType::Swap
            } else {
                format!("Futures-{}", parts[2]).as_str().try_into().unwrap()
            }
        }
        5 => format!("Options-{}-{}-{}", parts[2], parts[3], parts[4])
            .as_str()
            .try_into()
            .unwrap(),
        _ => unreachable!(),
    };
    Inst {
        exch: Exch::Okx,
        base_ccy,
        quote_ccy,
        inst_type,
    }
}

pub(super) fn str_to_naive_datetime(s: &str) -> NaiveDateTime {
    NaiveDateTime::from_timestamp_millis(s.parse().unwrap_or_default()).unwrap()
}

pub(super) fn str_to_mgn_mode(s: &str) -> MgnMode {
    match s {
        "cross" => MgnMode::Cross,
        "isolated" => MgnMode::Isolated,
        "cash" => MgnMode::Cash,
        _ => unreachable!(),
    }
}

pub(super) fn str_to_ord_type(s: &str) -> OrdType {
    match s {
        "market" => OrdType::Market,
        "limit" => OrdType::Limit,
        "post_only" => OrdType::PostOnly,
        "fok" => OrdType::Fok,
        "ioc" => OrdType::Ioc,
        _ => unreachable!(),
    }
}

pub(super) fn str_to_side(s: &str) -> Side {
    match s {
        "buy" => Side::Buy,
        "sell" => Side::Sell,
        _ => unreachable!(),
    }
}

pub(super) fn str_to_ord_state(s: &str) -> OrdState {
    match s {
        "live" => OrdState::Live,
        "filled" => OrdState::Filled,
        "canceled" => OrdState::Canceled,
        "partially_filled" => OrdState::PartiallyFilled,
        _ => unreachable!(),
    }
}
