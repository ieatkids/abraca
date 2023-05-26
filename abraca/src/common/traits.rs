use crate::common::{
    defs::{Inst, Result},
    msgs::{
        BalanceReport, CancelReject, Depth, ExecutionReport, Msg, MsgReceiver, MsgSender,
        PositionReport, Trade,
    },
};
use chrono::NaiveDateTime;

use super::msgs::{FundingRate, OpenInterest, Ticker};

pub trait Api {
    fn name(&self) -> &'static str;
    async fn start(self, tx: MsgSender, rx: MsgReceiver) -> Result<()>;
}

pub trait Strategy {
    fn on_depth(&mut self, depth: &Depth) -> Option<Msg>;
    fn on_trade(&mut self, trade: &Trade) -> Option<Msg>;
    fn on_ticker(&mut self, ticker: &Ticker) -> Option<Msg>;
    fn on_funding_rate(&mut self, rate: &FundingRate) -> Option<Msg>;
    fn on_open_interest(&mut self, interest: &OpenInterest) -> Option<Msg>;
    fn on_execution_report(&mut self, report: &ExecutionReport) -> Option<Msg>;
    fn on_cancel_reject(&mut self, reject: &CancelReject) -> Option<Msg>;
    fn on_balance_report(&mut self, report: &BalanceReport) -> Option<Msg>;
    fn on_position_report(&mut self, report: &PositionReport) -> Option<Msg>;
}

pub trait Feature {
    fn name(&self) -> &str;
    fn is_intrested(&self, inst: &Inst) -> bool;
    fn on_depth(&mut self, depth: &Depth);
    fn on_trade(&mut self, trade: &Trade);
    fn value(&self) -> Option<f64>;
    fn update_time(&self) -> NaiveDateTime;
}

pub trait FeatureLib {
    fn name(&self) -> &str;
    fn create_feature(&self, name: &str) -> Option<Box<dyn Feature>>;
}
