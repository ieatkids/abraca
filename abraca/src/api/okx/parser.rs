use crate::prelude::*;
use anyhow::Ok;
use chrono::NaiveDateTime;
use serde_json::Value;

pub fn parse_ticker(v: &Value) -> Result<Ticker> {
    Ok(Ticker {
        inst: str_to_inst(v["instId"].as_str().unwrap()),
        exch_time: str_to_naive_datetime(v["ts"].as_str().unwrap()),
        recv_time: chrono::Utc::now().naive_utc(),
        last: v["last"].as_str().unwrap().parse()?,
        last_sz: v["lastSz"].as_str().unwrap().parse()?,
        ask_px: v["askPx"].as_str().unwrap().parse()?,
        ask_sz: v["askSz"].as_str().unwrap().parse()?,
        bid_px: v["bidPx"].as_str().unwrap().parse()?,
        bid_sz: v["bidSz"].as_str().unwrap().parse()?,
    })
}

pub fn parse_funding_rate(v: &Value) -> Result<FundingRate> {
    Ok(FundingRate {
        inst: str_to_inst(v["instId"].as_str().unwrap()),
        recv_time: chrono::Utc::now().naive_utc(),
        funding_rate: v["fundingRate"].as_str().unwrap().parse()?,
        next_funding_rate: v["nextFundingRate"].as_str().unwrap().parse()?,
        funding_time: v["fundingTime"].as_str().unwrap().parse()?,
        next_funding_time: v["nextFundingTime"].as_str().unwrap().parse()?,
    })
}

pub fn parse_open_interest(v: &Value) -> Result<OpenInterest> {
    Ok(OpenInterest {
        inst: str_to_inst(v["instId"].as_str().unwrap()),
        exch_time: str_to_naive_datetime(v["ts"].as_str().unwrap()),
        recv_time: chrono::Utc::now().naive_utc(),
        oi: v["oi"].as_str().unwrap().parse()?,
        oi_ccy: v["oiCcy"].as_str().unwrap().parse()?,
    })
}

pub fn parse_books5(v: &Value) -> Result<Depth> {
    let mut asks = [(0.0, 0.0); 5];
    let mut bids = [(0.0, 0.0); 5];
    v["asks"]
        .as_array()
        .unwrap()
        .iter()
        .enumerate()
        .take(5)
        .for_each(|(i, a)| {
            asks[i] = (
                a[0].as_str().unwrap().parse().unwrap(),
                a[1].as_str().unwrap().parse().unwrap(),
            );
        });
    v["bids"]
        .as_array()
        .unwrap()
        .iter()
        .enumerate()
        .take(5)
        .for_each(|(i, b)| {
            bids[i] = (
                b[0].as_str().unwrap().parse().unwrap(),
                b[1].as_str().unwrap().parse().unwrap(),
            );
        });
    Ok(Depth {
        inst: str_to_inst(v["instId"].as_str().unwrap()),
        exch_time: str_to_naive_datetime(v["ts"].as_str().unwrap()),
        recv_time: chrono::Utc::now().naive_utc(),
        asks,
        bids,
    })
}

pub fn parse_trade(v: &Value) -> Result<Trade> {
    Ok(Trade {
        inst: str_to_inst(v["instId"].as_str().unwrap()),
        exch_time: str_to_naive_datetime(v["ts"].as_str().unwrap()),
        recv_time: chrono::Utc::now().naive_utc(),
        side: str_to_side(v["side"].as_str().unwrap()),
        px: v["px"].as_str().unwrap().parse()?,
        sz: v["sz"].as_str().unwrap().parse()?,
    })
}

pub fn parse_order(v: &Value) -> Result<ExecutionReport> {
    Ok(ExecutionReport {
        c_time: str_to_naive_datetime(v["cTime"].as_str().unwrap()),
        u_time: str_to_naive_datetime(v["uTime"].as_str().unwrap()),
        inst: str_to_inst(v["instId"].as_str().unwrap()),
        ord_id: v["ordId"].as_str().unwrap().parse()?,
        cl_ord_id: v["clOrdId"].as_str().unwrap().parse()?,
        px: v["px"].as_str().unwrap().parse()?,
        sz: v["sz"].as_str().unwrap().parse()?,
        notional_usd: v["notionalUsd"].as_str().unwrap().parse()?,
        ord_type: str_to_ord_type(v["ordType"].as_str().unwrap()),
        side: str_to_side(v["side"].as_str().unwrap()),
        fill_px: v["fillPx"].as_str().unwrap().parse()?,
        fill_sz: v["fillSz"].as_str().unwrap().parse()?,
        acc_fill_sz: v["accFillSz"].as_str().unwrap().parse()?,
        avg_px: v["avgPx"].as_str().unwrap().parse()?,
        state: str_to_ord_state(v["state"].as_str().unwrap()),
        lever: v["lever"].as_str().unwrap().parse()?,
        fee: v["fee"].as_str().unwrap().parse()?,
    })
}

pub fn parse_position(v: &Value) -> Result<PositionReport> {
    Ok(PositionReport {
        u_time: str_to_naive_datetime(v["uTime"].as_str().unwrap()),
        inst: str_to_inst(v["instId"].as_str().unwrap()),
        mgn_mode: str_to_mgn_mode(v["mgnMode"].as_str().unwrap()),
        pos: v["pos"].as_str().unwrap().parse()?,
        ccy: v["ccy"].as_str().unwrap().try_into()?,
        pos_ccy: v["posCcy"].as_str().unwrap().parse()?,
        avg_px: v["avgPx"].as_str().unwrap().parse()?,
    })
}

pub fn parse_balance_and_position(v: &Value) -> Result<BalanceReport> {
    Ok(BalanceReport {
        u_time: str_to_naive_datetime(v["uTime"].as_str().unwrap()),
        exch: Exch::Okx,
        ccy: v["ccy"].as_str().unwrap().try_into()?,
        cash_bal: v["cashBal"].as_str().unwrap().parse()?,
    })
}

fn str_to_inst(s: &str) -> Inst {
    let parts: Vec<&str> = s.split('-').collect();
    let base_ccy: Ccy = parts[0].try_into().unwrap_or_default();
    let quote_ccy: Ccy = parts[1].try_into().unwrap_or_default();
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

fn str_to_naive_datetime(s: &str) -> NaiveDateTime {
    NaiveDateTime::from_timestamp_millis(s.parse().unwrap_or_default()).unwrap()
}

fn str_to_mgn_mode(s: &str) -> MgnMode {
    match s {
        "cross" => MgnMode::Cross,
        "isolated" => MgnMode::Isolated,
        "cash" => MgnMode::Cash,
        _ => unreachable!(),
    }
}

fn str_to_ord_type(s: &str) -> OrdType {
    match s {
        "market" => OrdType::Market,
        "limit" => OrdType::Limit,
        "post_only" => OrdType::PostOnly,
        "fok" => OrdType::Fok,
        "ioc" => OrdType::Ioc,
        _ => unreachable!(),
    }
}

fn str_to_side(s: &str) -> Side {
    match s {
        "buy" => Side::Buy,
        "sell" => Side::Sell,
        _ => unreachable!(),
    }
}

fn str_to_ord_state(s: &str) -> OrdState {
    match s {
        "live" => OrdState::Live,
        "filled" => OrdState::Filled,
        "canceled" => OrdState::Canceled,
        "partially_filled" => OrdState::PartiallyFilled,
        _ => unreachable!(),
    }
}

pub fn inst_to_str(inst: &Inst) -> String {
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

pub fn td_mode_to_str(td_mode: &TdMode) -> &'static str {
    match td_mode {
        TdMode::Cross => "cross",
        TdMode::Isolated => "isolated",
        TdMode::Cash => "cash",
    }
}

pub fn side_to_str(side: &Side) -> &'static str {
    match side {
        Side::Buy => "buy",
        Side::Sell => "sell",
    }
}

pub fn ord_type_to_str(ord_type: &OrdType) -> &'static str {
    match ord_type {
        OrdType::Market => "market",
        OrdType::Limit => "limit",
        OrdType::PostOnly => "post_only",
        OrdType::Fok => "fok",
        OrdType::Ioc => "ioc",
    }
}
