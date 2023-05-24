use super::msgs::{
    BalanceReport, CancelReject, Depth, ExecutionReport, Msg, PositionReport, Trade,
};
use crate::prelude::{MsgReceiver, MsgSender};
use chrono::NaiveDateTime;

pub trait Api {
    fn name(&self) -> &'static str;
    async fn start(self, tx: MsgSender, rx: MsgReceiver);
}

pub trait Strategy {
    fn on_depth(&mut self, depth: &Depth) -> Option<Msg>;
    fn on_trade(&mut self, trade: &Trade) -> Option<Msg>;
    fn on_execution_report(&mut self, report: &ExecutionReport) -> Option<Msg>;
    fn on_cancel_reject(&mut self, reject: &CancelReject) -> Option<Msg>;
    fn on_balance_report(&mut self, report: &BalanceReport) -> Option<Msg>;
    fn on_position_report(&mut self, report: &PositionReport) -> Option<Msg>;
}

pub trait Feature {
    fn name(&self) -> &str;
    fn on_depth(&mut self, depth: &Depth);
    fn on_trade(&mut self, trade: &Trade);
    fn value(&self) -> Option<f64>;
    fn update_time(&self) -> NaiveDateTime;
}
