use crate::common::defs::{Ccy, Exch, Inst, MgnMode, OrdState, OrdType, Side, TdMode};
use chrono::NaiveDateTime;
use serde::{Deserialize, Serialize};
use tokio::sync::mpsc::{Receiver, Sender};

pub type MsgSender = Sender<Msg>;
pub type MsgReceiver = Receiver<Msg>;

pub fn new_channel(buffer: usize) -> (MsgSender, MsgReceiver) {
    tokio::sync::mpsc::channel(buffer)
}

#[repr(align(8))]
#[non_exhaustive]
#[derive(Debug, Clone, Deserialize, Serialize)]
pub enum Msg {
    Depth(Depth),
    Trade(Trade),
    Ticker(Ticker),
    FundingRate(FundingRate),
    OpenInterest(OpenInterest),
    NewOrder(NewOrder),
    CancelOrder(CancelOrder),
    ExecutionReport(ExecutionReport),
    CancelReject(CancelReject),
    BalanceReport(BalanceReport),
    PositionReport(PositionReport),
    SigTerm,
}

#[derive(Debug, Clone, Deserialize, Serialize)]
pub struct Depth {
    /// instrument id
    pub inst: Inst,
    /// exchange time
    pub exch_time: NaiveDateTime,
    /// receive time
    pub recv_time: NaiveDateTime,
    /// ask prices and sizes
    pub asks: [(f64, f64); 5],
    /// bid prices and sizes
    pub bids: [(f64, f64); 5],
}

#[derive(Debug, Clone, Deserialize, Serialize)]
pub struct Trade {
    /// instrument id
    pub inst: Inst,
    /// exchange time
    pub exch_time: NaiveDateTime,
    /// receive time
    pub recv_time: NaiveDateTime,
    /// trade side
    pub side: Side,
    /// trade price
    pub px: f64,
    /// trade size
    pub sz: f64,
}

#[derive(Debug, Clone, Deserialize, Serialize)]
pub struct Ticker {
    /// instrument id
    pub inst: Inst,
    /// exchange time
    pub exch_time: NaiveDateTime,
    /// receive time
    pub recv_time: NaiveDateTime,
    /// last price
    pub last: f64,
    /// last size
    pub last_sz: f64,
    /// best ask price
    pub ask_px: f64,
    /// best ask size
    pub ask_sz: f64,
    /// best bid price
    pub bid_px: f64,
    /// best bid size
    pub bid_sz: f64,
}

#[derive(Debug, Clone, Deserialize, Serialize)]
pub struct FundingRate {
    /// instrument id
    pub inst: Inst,
    /// receive time
    pub recv_time: NaiveDateTime,
    /// funding rate
    pub funding_rate: f64,
    /// next funding rate
    pub next_funding_rate: f64,
    /// funding time
    pub funding_time: NaiveDateTime,
    /// next funding time
    pub next_funding_time: NaiveDateTime,
}

#[derive(Debug, Clone, Deserialize, Serialize)]
pub struct OpenInterest {
    /// instrument id
    pub inst: Inst,
    /// exchange time
    pub exch_time: NaiveDateTime,
    /// receive time
    pub recv_time: NaiveDateTime,
    /// open interest
    pub oi: f64,
    /// open interest in currency
    pub oi_ccy: f64,
}

#[derive(Debug, Clone, Deserialize, Serialize)]
pub struct NewOrder {
    /// instrument id
    pub inst: Inst,
    /// client order id
    pub cl_ord_id: i64,
    /// order side
    pub side: Side,
    /// order type
    pub ord_type: OrdType,
    /// trade mode
    pub td_mode: TdMode,
    /// order price
    pub px: f64,
    /// order size
    pub sz: f64,
}

#[derive(Debug, Clone, Deserialize, Serialize)]
pub struct CancelOrder {
    /// instrument id
    pub inst: Inst,
    /// client order id
    pub cl_ord_id: i64,
}

#[derive(Debug, Clone, Deserialize, Serialize)]
pub struct ExecutionReport {
    /// creation timestamp
    pub c_time: NaiveDateTime,
    /// update timestamp
    pub u_time: NaiveDateTime,
    /// instrument id
    pub inst: Inst,
    /// order id
    pub ord_id: i64,
    /// client order id
    pub cl_ord_id: i64,
    /// order price
    pub px: f64,
    /// order size
    pub sz: f64,
    /// order notional in USD
    pub notional_usd: f64,
    /// order type
    pub ord_type: OrdType,
    /// order side
    pub side: Side,
    /// last filled price
    pub fill_px: f64,
    /// last filled size
    pub fill_sz: f64,
    /// accumulated filled size
    pub acc_fill_sz: f64,
    /// average filled price
    pub avg_px: f64,
    /// order state
    pub state: OrdState,
    /// leverage. only for [`InstType::MARGIN`/`InstType::FUTURES`/`InstType::SWAP`]
    pub lever: f64,
    /// fee and rebate
    pub fee: f64,
}

#[derive(Debug, Clone, Deserialize, Serialize)]
pub struct CancelReject {
    /// update timestamp
    pub u_time: NaiveDateTime,
    /// instrument id
    pub inst: Inst,
    /// client order id
    pub cl_ord_id: i64,
}

#[derive(Debug, Clone, Deserialize, Serialize)]
pub struct BalanceReport {
    /// update time
    pub u_time: NaiveDateTime,
    /// exchange
    pub exch: Exch,
    /// currency
    pub ccy: Ccy,
    /// cash balance
    pub cash_bal: f64,
}

#[derive(Debug, Clone, Deserialize, Serialize)]
pub struct PositionReport {
    /// update time
    pub u_time: NaiveDateTime,
    /// instrument id
    pub inst: Inst,
    /// margin mode
    pub mgn_mode: MgnMode,
    /// position
    pub pos: f64,
    /// currency used for margin
    pub ccy: Ccy,
    /// position currency, only for `InstType::MARGIN`
    pub pos_ccy: Ccy,
    /// average open price
    pub avg_px: f64,
}
