use abraca::{api::okx::OkxApi, prelude::*};

struct Logger;

#[allow(unused_variables)]
impl Strategy for Logger {
    fn on_depth(&mut self, depth: &Depth) -> Option<Msg> {
        None
    }

    fn on_trade(&mut self, trade: &Trade) -> Option<Msg> {
        println!("{:?}", trade);
        None
    }
    fn on_ticker(&mut self, ticker: &Ticker) -> Option<Msg> {
        println!("{:?}", ticker);
        None
    }

    fn on_funding_rate(&mut self, rate: &FundingRate) -> Option<Msg> {
        println!("{:?}", rate);
        None
    }

    fn on_open_interest(&mut self, interest: &OpenInterest) -> Option<Msg> {
        println!("{:?}", interest);
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

fn main() -> Result<()> {
    env_logger::builder()
        .filter_level(log::LevelFilter::Info)
        .try_init()?;
    let api = OkxApi::builder()
        .subscribe([
            ("Okx.BTC.USD.Swap", "Ticker"),
            ("Okx.BTC.USD.Swap", "OpenInterest"),
            ("Okx.BTC.USD.Swap", "FundingRate"),
            ("Okx.BTC.USD.Swap", "Depth"),
        ])
        .subscribe([(("Okx", "ETH", "USD", "Swap"), "Depth")])
        .build();
    let stg = Logger;
    abraca::utils::run_stg(api, stg)
}
