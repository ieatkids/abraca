use crate::prelude::*;

pub mod dingtalk;

struct Wrapper<S: Strategy>(S);

impl<S: Strategy> Wrapper<S> {
    async fn start(mut self, tx: MsgSender, mut rx: MsgReceiver) -> Result<()> {
        log::info!("start strategy");
        while let Some(resp) = rx.recv().await {
            if let Some(req) = match resp {
                Msg::Depth(d) => self.0.on_depth(&d),
                Msg::Trade(d) => self.0.on_trade(&d),
                Msg::Ticker(d) => self.0.on_ticker(&d),
                Msg::FundingRate(d) => self.0.on_funding_rate(&d),
                Msg::OpenInterest(d) => self.0.on_open_interest(&d),
                Msg::ExecutionReport(d) => self.0.on_execution_report(&d),
                Msg::CancelReject(d) => self.0.on_cancel_reject(&d),
                Msg::BalanceReport(d) => self.0.on_balance_report(&d),
                Msg::PositionReport(d) => self.0.on_position_report(&d),
                _ => None,
            } {
                tx.send(req).await?;
            }
        }
        Ok(())
    }
}

pub fn run_stg<A: Api, S: Strategy>(api: A, stg: S) -> Result<()> {
    tokio::runtime::Builder::new_current_thread()
        .enable_all()
        .build()
        .unwrap()
        .block_on(async {
            let (s_tx, a_rx) = new_channel(1024);
            let (a_tx, s_rx) = new_channel(1024);
            let (res1, res2) = tokio::join!(api.start(a_tx, a_rx), Wrapper(stg).start(s_tx, s_rx));
            res1?;
            res2
        })
}
