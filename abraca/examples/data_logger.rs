use abraca::{api::okx::OkxApi, prelude::*};

struct Logger;

#[allow(unused_variables)]
impl Strategy for Logger {
    fn on_depth(&mut self, depth: &Depth) -> Option<Msg> {
        println!("{:?}", depth);
        None
    }

    fn on_trade(&mut self, trade: &Trade) -> Option<Msg> {
        println!("{:?}", trade);
        None
    }

    fn on_execution_report(&mut self, report: &ExecutionReport) -> Option<Msg> {
        None
    }

    fn on_cancel_reject(&mut self, reject: &CancelReject) -> Option<Msg> {
        None
    }

    fn on_balance_report(&mut self, report: &BalanceReport) -> Option<Msg> {
        None
    }

    fn on_position_report(&mut self, report: &PositionReport) -> Option<Msg> {
        None
    }
}

#[tokio::main]
async fn main() {
    let api = OkxApi::builder()
        .subscribe([("BTC", "USDT", "Spot"), ("ETH", "USDT", "Swap")])
        .build();
    let (_, a_rx) = new_channel(10);
    let (a_tx, mut s_rx) = new_channel(10);
    api.start(a_tx, a_rx).await;
    while let Some(m) = s_rx.recv().await {
        println!("{:?}", m);
    }
}
