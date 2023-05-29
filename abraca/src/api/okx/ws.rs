use super::parser;
use crate::prelude::*;
use base64::{engine::general_purpose, Engine};
use chrono::Utc;
use futures::{SinkExt, StreamExt};
use ring::hmac;
use serde::{Deserialize, Serialize};
use serde_json::Value;
use serde_with::{serde_as, DisplayFromStr};
use std::collections::HashMap;
use tokio_tungstenite::{connect_async, tungstenite::Message};

#[cfg(feature = "testnet")]
const PUBLIC_WS_URL: &str = "wss://wspap.okx.com:8443/ws/v5/public?brokerId=9999";

#[cfg(feature = "testnet")]
const PRIVATE_WS_URL: &str = "wss://wspap.okx.com:8443/ws/v5/private?brokerId=9999";

#[cfg(not(feature = "testnet"))]
const PUBLIC_WS_URL: &str = "wss://ws.okx.com:8443/ws/v5/public";

#[cfg(not(feature = "testnet"))]
const PRIVATE_WS_URL: &str = "wss://ws.okx.com:8443/ws/v5/private";

fn get_sign(ts: &str, method: &str, path: &str, body: &str, secretkey: &str) -> String {
    let key = hmac::Key::new(hmac::HMAC_SHA256, secretkey.as_bytes());
    let sign = hmac::sign(&key, format!("{ts}{method}{path}{body}").as_bytes());
    general_purpose::STANDARD.encode(sign)
}

#[non_exhaustive]
#[derive(Debug, PartialEq, Deserialize, Serialize)]
#[serde(rename_all = "kebab-case")]
pub enum WsChannel {
    Positions,
    #[serde(rename = "balance_and_position")]
    BalanceAndPosition,
    Orders,
    Tickers,
    FundingRate,
    OpenInterest,
    Books5,
    Trade,
}

#[derive(Debug, Deserialize, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct WsChannelArg {
    pub channel: WsChannel,
    #[serde(default)]
    pub inst_id: Option<String>,
    #[serde(default)]
    pub inst_type: Option<String>,
    #[serde(default)]
    pub inst_family: Option<String>,
}

#[derive(Debug, Deserialize, Serialize)]
#[serde(rename_all = "camelCase")]
struct WsAccount {
    api_key: String,
    passphrase: String,
    timestamp: String,
    sign: String,
}

#[derive(Debug, Deserialize, Serialize)]
struct WsOrder {
    inst_id: String,
    td_mode: String,
    cl_ord_id: String,
    side: String,
    ord_type: String,
    sz: f64,
    px: f64,
}

#[derive(Debug, Deserialize, Serialize)]
struct WsCancel {
    inst_id: String,
    cl_ord_id: String,
}

#[derive(Serialize)]
#[serde(tag = "op", rename_all = "snake_case")]
enum WsCommand {
    Login { args: Vec<WsAccount> },
    Subscribe { args: Vec<WsChannelArg> },
    Order { id: String, args: Vec<WsOrder> },
    CancelOrder { id: String, args: Vec<WsCancel> },
}

#[derive(Debug, Deserialize, Serialize)]
#[serde(untagged)]
enum WsMessage {
    Data { data: Vec<Value>, arg: WsChannelArg },
    TradeResult(TradeResult),
    LoginResult(LoginResult),
    SubscribeResult { arg: WsChannelArg },
}

#[derive(Debug, PartialEq, Deserialize, Serialize)]
#[serde(rename_all = "kebab-case")]
enum WsOp {
    Order,
    CancelOrder,
}

#[derive(Debug, Deserialize, Serialize)]
#[serde(rename_all = "lowercase")]
enum WsEvent {
    Login,
    Error,
}

#[serde_as]
#[derive(Debug, Deserialize, Serialize)]
struct TradeResult {
    #[serde_as(as = "DisplayFromStr")]
    id: i64,
    op: WsOp,
    #[serde_as(as = "DisplayFromStr")]
    code: i64,
    msg: String,
}

impl TradeResult {
    fn is_ok(&self) -> bool {
        self.code == 0
    }
}

#[serde_as]
#[derive(Debug, Deserialize, Serialize)]
struct LoginResult {
    event: WsEvent,
    #[serde_as(as = "DisplayFromStr")]
    code: i64,
    msg: String,
}

impl LoginResult {
    fn is_ok(&self) -> bool {
        self.code == 0
    }
}

pub struct PublicClient {
    pub channels: Vec<WsChannelArg>,
}

impl PublicClient {
    pub async fn start(self, tx: MsgSender) -> Result<()> {
        let (mut ws, _) = connect_async(PUBLIC_WS_URL).await.unwrap();
        log::info!("connected to okx public websocket");
        let cmd = WsCommand::Subscribe {
            args: self.channels,
        };
        ws.send(Message::Text(serde_json::to_string(&cmd)?)).await?;
        log::info!("send subscribe request");
        while let Some(msg) = ws.next().await {
            if let Message::Text(payload) = msg? {
                let ws_msg: WsMessage = serde_json::from_str(&payload)?;
                match ws_msg {
                    WsMessage::Data { data, arg } => match arg.channel {
                        WsChannel::Tickers => {
                            for d in data {
                                if let Ok(m) = parser::parse_ticker(&d) {
                                    tx.send(Msg::Ticker(m)).await?;
                                }
                            }
                        }
                        WsChannel::FundingRate => {
                            for d in data {
                                if let Ok(m) = parser::parse_funding_rate(&d) {
                                    tx.send(Msg::FundingRate(m)).await?;
                                }
                            }
                        }
                        WsChannel::OpenInterest => {
                            for d in data {
                                if let Ok(m) = parser::parse_open_interest(&d) {
                                    tx.send(Msg::OpenInterest(m)).await?;
                                }
                            }
                        }
                        WsChannel::Books5 => {
                            for d in data {
                                if let Ok(m) = parser::parse_books5(&d) {
                                    tx.send(Msg::Depth(m)).await?;
                                }
                            }
                        }
                        WsChannel::Trade => {
                            for d in data {
                                if let Ok(m) = parser::parse_trade(&d) {
                                    tx.send(Msg::Trade(m)).await?;
                                }
                            }
                        }
                        _ => (),
                    },
                    WsMessage::SubscribeResult { arg } => {
                        log::info!("subscribe succeed. {:?} {:?}", arg.inst_id, arg.channel);
                    }
                    _ => log::error!("unexpected message: {:?}", ws_msg),
                }
            }
        }
        Ok(())
    }
}

pub struct PrivateClient {
    pub apikey: String,
    pub secretkey: String,
    pub passphrase: String,
    pub channels: Vec<WsChannelArg>,
}

impl PrivateClient {
    pub async fn start(self, tx: MsgSender, mut rx: MsgReceiver) -> Result<()> {
        let (ws, _) = connect_async(PRIVATE_WS_URL).await.unwrap();
        let (mut write, mut read) = ws.split();
        log::info!("connected to private websocket");
        let timestamp = chrono::Utc::now().timestamp().to_string();
        let sign = get_sign(&timestamp, "GET", "/users/self/verify", "", &self.secretkey);
        let login_cmd = WsCommand::Login {
            args: vec![WsAccount {
                api_key: self.apikey,
                passphrase: self.passphrase,
                timestamp,
                sign,
            }],
        };
        write
            .send(Message::Text(serde_json::to_string(&login_cmd)?))
            .await?;
        log::info!("sent login request");
        let sub_cmd = WsCommand::Subscribe {
            args: self.channels,
        };
        let mut order_cache = HashMap::<i64, NewOrder>::new();
        let mut cancel_cache = HashMap::<i64, CancelOrder>::new();
        loop {
            tokio::select! {
                m = rx.recv() => {
                    if let Some(m) = m {
                        let id = Utc::now().timestamp();
                        match m {
                            Msg::NewOrder(o) => {
                                let ws_order = WsOrder {
                                    inst_id: parser::inst_to_str(&o.inst),
                                    td_mode: parser::td_mode_to_str(&o.td_mode).to_owned(),
                                    cl_ord_id: id.to_string(),
                                    side: parser::side_to_str(&o.side).to_owned(),
                                    ord_type: parser::ord_type_to_str(&o.ord_type).to_owned(),
                                    sz: o.sz,
                                    px: o.px,
                                };
                                let cmd = WsCommand::Order { id: id.to_string(), args: vec![ws_order] };
                                write.send(Message::Text(serde_json::to_string(&cmd)?)).await?;
                                order_cache.insert(id, o);
                            },
                            Msg::CancelOrder(c) => {
                                let ws_cancel = WsCancel {
                                    inst_id: parser::inst_to_str(&c.inst),
                                    cl_ord_id: c.cl_ord_id.to_string(),
                                };
                                let cmd = WsCommand::CancelOrder { id: id.to_string(), args: vec![ws_cancel] };
                                write.send(Message::Text(serde_json::to_string(&cmd)?)).await?;
                                cancel_cache.insert(id, c);
                            },
                            _ => (),
                        }
                    }
                },
                m = read.next() => {
                    if let Some(m) = m{
                        if let Message::Text(m) = m? {
                            let ws_msg: WsMessage = serde_json::from_str(&m)?;
                            match ws_msg {
                                WsMessage::LoginResult(res) => {
                                    if res.is_ok() {
                                        log::info!("okx private websocket login succeed");
                                        write
                                            .send(Message::Text(serde_json::to_string(&sub_cmd)?))
                                            .await?;
                                    } else {
                                        log::error!("okx private websocket login failed: {}", res.msg);
                                    }
                                }
                                WsMessage::Data { data, arg } => {
                                    match arg.channel {
                                        WsChannel::Orders => {
                                            for d in data {
                                                if let Ok(m) = parser::parse_order(&d) {
                                                    tx.send(Msg::ExecutionReport(m)).await?;
                                                }
                                            }
                                        }
                                        WsChannel::Positions => {
                                            for d in data {
                                                if let Ok(m) = parser::parse_position(&d) {
                                                    tx.send(Msg::PositionReport(m)).await?;
                                                }
                                            }
                                        }
                                        WsChannel::BalanceAndPosition => {
                                            for d in data{
                                                for b in d["balData"].as_array().unwrap(){
                                                    if let Ok(m) = parser::parse_balance_and_position(b){
                                                        tx.send(Msg::BalanceReport(m)).await?;
                                                    }
                                                }
                                            }
                                        }
                                        _ => (),
                                    }
                                }
                                WsMessage::SubscribeResult { arg } => {
                                    log::info!("subscribe succeed. {:?} {:?}", arg.inst_id, arg.channel);
                                },
                                WsMessage::TradeResult(res) => {
                                    match res.op{
                                        WsOp::Order => {
                                            if res.is_ok(){
                                                let _ = order_cache.remove(&res.id);
                                            }else if let Some(o) = order_cache.remove(&res.id){
                                                let ts = chrono::Utc::now().naive_utc();
                                                let m = ExecutionReport{
                                                    c_time: ts,
                                                    u_time: ts,
                                                    inst: o.inst,
                                                    ord_id: 0,
                                                    cl_ord_id: o.cl_ord_id,
                                                    px: o.px,
                                                    sz: o.sz,
                                                    notional_usd: 0.0,
                                                    ord_type: o.ord_type,
                                                    side: o.side,
                                                    fill_px: 0.0,
                                                    fill_sz: 0.0,
                                                    acc_fill_sz: 0.0,
                                                    avg_px: 0.0,
                                                    state: OrdState::Rejected,
                                                    lever: 0.0,
                                                    fee: 0.0,
                                                };
                                                tx.send(Msg::ExecutionReport(m)).await?;
                                            }else{
                                                log::warn!("order not found: {}", res.id);
                                            }
                                        },
                                        WsOp::CancelOrder => {
                                            if res.is_ok() {
                                                let _ = order_cache.remove(&res.id);
                                            } else if let Some(c) = cancel_cache.remove(&res.id){
                                                let m = CancelReject{
                                                    inst: c.inst,
                                                    cl_ord_id: c.cl_ord_id,
                                                    u_time: chrono::Utc::now().naive_utc(),
                                                };
                                                tx.send(Msg::CancelReject(m)).await?;
                                            }else{
                                                log::warn!("cancel not found: {}", res.id);
                                            }
                                        },
                                    }
                                },
                            }
                        }
                    }
                }
            }
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn deserialize_login_succeed_msg_works() {
        let s = r#"{
            "event": "login",
            "code": "0",
            "msg": ""
        }"#;
        let m: WsMessage = serde_json::from_str(s).unwrap();
        match m {
            WsMessage::LoginResult(res) => {
                assert!(res.is_ok());
            }
            _ => panic!("unexpected message"),
        };
    }

    #[test]
    fn deserialize_login_failed_msg_works() {
        let s = r#"
        {
            "event": "error",
            "code": "60009",
            "msg": "Login failed."
        }"#;

        let m: WsMessage = serde_json::from_str(s).unwrap();
        match m {
            WsMessage::LoginResult(res) => {
                assert!(!res.is_ok());
            }
            _ => panic!("unexpected message"),
        };
    }

    #[test]
    fn deserialize_subscribe_succeed_msg_works() {
        let s = r#"
        {
            "event": "subscribe",
            "arg": {
                "channel": "tickers",
                "instId": "LTC-USD-200327"
            }
        }"#;

        let m: WsMessage = serde_json::from_str(s).unwrap();
        match m {
            WsMessage::SubscribeResult { arg } => {
                assert_eq!(arg.channel, WsChannel::Tickers);
                assert_eq!(arg.inst_id.unwrap(), "LTC-USD-200327");
            }
            _ => panic!("unexpected message"),
        };
    }

    #[test]
    fn deserialize_order_succeed_msg_works() {
        let s = r#"
        {
            "id": "1514",
            "op": "cancel-order",
            "data": [{
                "clOrdId": "",
                "ordId": "2510789768709120",
                "sCode": "0",
                "sMsg": ""
            }],
            "code": "0",
            "msg": ""
        }"#;
        let m: WsMessage = serde_json::from_str(s).unwrap();
        match m {
            WsMessage::TradeResult(res) => {
                assert!(res.is_ok());
                assert_eq!(res.id, 1514);
                assert_eq!(res.op, WsOp::CancelOrder);
            }
            _ => panic!("unexpected message"),
        };
    }

    #[test]
    fn deserialize_order_failed_msg_works() {
        let s = r#"
        {
            "id": "1512",
            "op": "order",
            "data": [{
                "clOrdId": "",
                "ordId": "",
                "tag": "",
                "sCode": "5XXXX",
                "sMsg": "not exist"
            }],
            "code": "1",
            "msg": ""
        }"#;
        let m: WsMessage = serde_json::from_str(s).unwrap();
        match m {
            WsMessage::TradeResult(res) => {
                assert!(!res.is_ok());
                assert_eq!(res.id, 1512);
                assert_eq!(res.op, WsOp::Order);
            }
            _ => panic!("unexpected message"),
        };

        let s = r#"
        {
            "id": "1512",
            "op": "order",
            "data": [],
            "code": "60013",
            "msg": "Invalid args"
        }"#;
        let m: WsMessage = serde_json::from_str(s).unwrap();
        match m {
            WsMessage::TradeResult(res) => {
                assert!(!res.is_ok());
                assert_eq!(res.id, 1512);
                assert_eq!(res.op, WsOp::Order);
            }
            _ => panic!("unexpected message"),
        };
    }

    #[test]
    fn deserialize_cancel_succeed_msg_works() {
        let s = r#"
        {
            "id": "1514",
            "op": "cancel-order",
            "data": [{
                "clOrdId": "",
                "ordId": "2510789768709120",
                "sCode": "0",
                "sMsg": ""
            }],
            "code": "0",
            "msg": ""
        }"#;
        let m: WsMessage = serde_json::from_str(s).unwrap();
        match m {
            WsMessage::TradeResult(res) => {
                assert!(res.is_ok());
                assert_eq!(res.id, 1514);
                assert_eq!(res.op, WsOp::CancelOrder);
            }
            _ => panic!("unexpected message"),
        };
    }

    #[test]
    fn deserialize_cancel_failed_msg_works() {
        let s = r#"
        {
            "id": "1514",
            "op": "cancel-order",
            "data": [{
                "clOrdId": "",
                "ordId": "2510789768709120",
                "sCode": "5XXXX",
                "sMsg": "Order not exist"
            }],
            "code": "1",
            "msg": ""
        }"#;
        let m: WsMessage = serde_json::from_str(s).unwrap();
        match m {
            WsMessage::TradeResult(res) => {
                assert!(!res.is_ok());
                assert_eq!(res.id, 1514);
                assert_eq!(res.op, WsOp::CancelOrder);
            }
            _ => panic!("unexpected message"),
        };

        let s = r#"
        {
            "id": "1514",
            "op": "cancel-order",
            "data": [],
            "code": "60013",
            "msg": "Invalid args"
        }"#;
        let m: WsMessage = serde_json::from_str(s).unwrap();
        match m {
            WsMessage::TradeResult(res) => {
                assert!(!res.is_ok());
                assert_eq!(res.id, 1514);
                assert_eq!(res.op, WsOp::CancelOrder);
            }
            _ => panic!("unexpected message"),
        };
    }
}
