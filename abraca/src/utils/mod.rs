use crate::prelude::*;

pub mod dingtalk;

/// Run a strategy with a given api.
pub fn run_stg<A: Api, S: Strategy>(api: A, stg: S) -> Result<()> {
    tokio::runtime::Builder::new_current_thread()
        .enable_all()
        .build()
        .unwrap()
        .block_on(async { run_stg_async(api, stg).await })
}

pub async fn run_stg_async<A: Api, S: Strategy>(api: A, mut stg: S) -> Result<()> {
    let (s_tx, a_rx) = new_channel(1024);
    let (a_tx, mut s_rx) = new_channel(1024);
    log::info!("start api: {}", api.name());
    api.start(a_tx, a_rx).await;
    while let Some(m) = s_rx.recv().await {
        match m {
            Msg::Depth(m) => {
                if let Some(msg) = stg.on_depth(&m) {
                    s_tx.send(msg).await?;
                }
            }
            Msg::Trade(m) => {
                if let Some(msg) = stg.on_trade(&m) {
                    s_tx.send(msg).await?;
                }
            }
            Msg::ExecutionReport(m) => {
                if let Some(msg) = stg.on_execution_report(&m) {
                    s_tx.send(msg).await?;
                }
            }
            Msg::CancelReject(m) => {
                if let Some(msg) = stg.on_cancel_reject(&m) {
                    s_tx.send(msg).await?;
                }
            }
            Msg::BalanceReport(m) => {
                if let Some(msg) = stg.on_balance_report(&m) {
                    s_tx.send(msg).await?;
                }
            }
            Msg::PositionReport(m) => {
                if let Some(msg) = stg.on_position_report(&m) {
                    s_tx.send(msg).await?;
                }
            }
            _ => (),
        }
    }
    Ok(())
}
