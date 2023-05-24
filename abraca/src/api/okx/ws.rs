use super::{get_sign, OkxCredential};
use crate::{api::okx::parser, prelude::*};
use futures::{SinkExt, StreamExt};
use serde::{Deserialize, Serialize};
use serde_json::Value;
use std::collections::HashSet;
use tokio_tungstenite::{connect_async, tungstenite::Message};

#[cfg(feature = "testnet")]
const PUBLIC_WS_URL: &str = "wss://wspap.okx.com:8443/ws/v5/public?brokerId=9999";

#[cfg(feature = "testnet")]
const PRIVATE_WS_URL: &str = "wss://wspap.okx.com:8443/ws/v5/private?brokerId=9999";

#[cfg(not(feature = "testnet"))]
const PUBLIC_WS_URL: &str = "wss://ws.okx.com:8443/ws/v5/public";

#[cfg(not(feature = "testnet"))]
const PRIVATE_WS_URL: &str = "wss://ws.okx.com:8443/ws/v5/private";

#[derive(Debug, Serialize)]
#[serde(tag = "op", content = "args", rename_all = "snake_case")]
enum WsCommand {
    Subscribe(Vec<WsChannel>),
    Login(Vec<LoginArg>),
}

#[derive(Debug, Serialize)]
#[serde(rename_all = "camelCase")]
struct LoginArg {
    api_key: String,
    passphrase: String,
    timestamp: String,
    sign: String,
}

#[derive(Debug, Clone, Deserialize, Serialize)]
#[serde(tag = "channel")]
enum WsChannel {
    #[serde(rename = "books5")]
    Books {
        #[serde(rename = "instId")]
        inst_id: String,
    },
    #[serde(rename = "trades")]
    Trades {
        #[serde(rename = "instId")]
        inst_id: String,
    },
    #[serde(rename = "balance_and_position")]
    BalanceAndPosition,
    #[serde(rename = "positions")]
    Positions {
        #[serde(rename = "instType")]
        inst_type: String,
    },
    #[serde(rename = "orders")]
    Orders {
        #[serde(rename = "instType")]
        inst_type: String,
    },
}

#[derive(Clone, Debug, Deserialize, Serialize)]
#[serde(untagged)]
enum WsMessage {
    Data { arg: WsChannel, data: Vec<Value> },
    LoginResult { code: String, msg: String },
    SubscribeResult { arg: WsChannel },
    Pong,
}

pub(super) struct PublicWsClient {
    subs: Vec<Inst>,
}

impl PublicWsClient {
    pub fn new(subs: &[Inst]) -> Self {
        Self {
            subs: subs.to_vec(),
        }
    }

    pub async fn run(self, tx: MsgSender) {
        let (mut ws, _) = connect_async(PUBLIC_WS_URL).await.unwrap();
        log::info!("connected to public websocket");
        let channel: Vec<_> = self
            .subs
            .iter()
            .map(|i| WsChannel::Books {
                inst_id: parser::inst_to_str(i),
            })
            .collect();
        let cmd = WsCommand::Subscribe(channel);
        ws.send(Message::Text(serde_json::to_string(&cmd).unwrap()))
            .await
            .unwrap();
        while let Some(msg) = ws.next().await {
            if let Message::Text(payload) = msg.unwrap() {
                let ws_msg: WsMessage = serde_json::from_str(&payload).unwrap();
                match ws_msg {
                    WsMessage::SubscribeResult { arg } => {
                        log::info!("subscribed to {}", serde_json::to_string(&arg).unwrap());
                    }
                    WsMessage::Data { arg, data } => match arg {
                        WsChannel::Books { inst_id: _ } => {
                            for v in data {
                                let depth = deserialize_books5(&v);
                                tx.send(Msg::Depth(depth)).await.unwrap();
                            }
                        }
                        WsChannel::Trades { inst_id: _ } => {
                            for v in data {
                                let trade = deserialize_trades(&v);
                                tx.send(Msg::Trade(trade)).await.unwrap();
                            }
                        }
                        _ => (),
                    },
                    _ => (),
                }
            }
        }
    }
}

pub(super) struct PrivateWsClient {
    apikey: String,
    secretkey: String,
    passphrase: String,
    subs: Vec<Inst>,
}

impl PrivateWsClient {
    pub fn new(subs: &[Inst], credential: &OkxCredential) -> Self {
        Self {
            apikey: credential.apikey.to_owned(),
            secretkey: credential.secretkey.to_owned(),
            passphrase: credential.passphrase.to_owned(),
            subs: subs.to_vec(),
        }
    }

    pub async fn run(self, tx: MsgSender) {
        let (ws, _) = connect_async(PRIVATE_WS_URL).await.unwrap();
        let (mut write, mut read) = ws.split();
        log::info!("connected to private websocket");

        let ts = chrono::Utc::now().timestamp().to_string();
        let login_cmd = WsCommand::Login(vec![LoginArg {
            api_key: self.apikey.clone(),
            passphrase: self.passphrase.clone(),
            timestamp: ts.clone(),
            sign: get_sign(&ts, "GET", "/users/self/verify", "", &self.secretkey),
        }]);

        let inst_types: HashSet<_> = self
            .subs
            .iter()
            .map(|i| parser::inst_type_to_str(&i.inst_type))
            .collect();

        let mut channels: Vec<_> = inst_types
            .iter()
            .map(|t| WsChannel::Positions {
                inst_type: (*t).to_owned(),
            })
            .chain(inst_types.iter().map(|t| WsChannel::Orders {
                inst_type: (*t).to_owned(),
            }))
            .collect();
        channels.push(WsChannel::BalanceAndPosition);
        let sub_cmd = WsCommand::Subscribe(channels);

        log::info!("login to private websocket");
        write
            .send(Message::Text(serde_json::to_string(&login_cmd).unwrap()))
            .await
            .unwrap();

        while let Some(msg) = read.next().await {
            if let Message::Text(payload) = msg.unwrap() {
                let ws_msg: WsMessage = serde_json::from_str(&payload).unwrap();

                match ws_msg {
                    WsMessage::LoginResult { code, msg } => {
                        if code == "0" {
                            log::info!(
                                "Login to private websocket success. subscribe data channels"
                            );
                            write
                                .send(Message::Text(serde_json::to_string(&sub_cmd).unwrap()))
                                .await
                                .unwrap();
                        } else {
                            log::error!("Login to private websocket failed. {}: {}", code, msg);
                            return;
                        }
                    }
                    WsMessage::SubscribeResult { arg } => {
                        log::info!("Subscribed to {}", serde_json::to_string(&arg).unwrap());
                    }
                    WsMessage::Data { arg, data } => match arg {
                        WsChannel::BalanceAndPosition => {
                            for v in data {
                                for br in deserialize_balance_and_position(&v) {
                                    tx.send(Msg::BalanceReport(br)).await.unwrap();
                                }
                            }
                        }
                        WsChannel::Positions { inst_type: _ } => {
                            for v in data {
                                let pr = deserialize_positions(&v);
                                tx.send(Msg::PositionReport(pr)).await.unwrap();
                            }
                        }
                        WsChannel::Orders { inst_type: _ } => {
                            for v in data {
                                let er = deserialize_orders(&v);
                                tx.send(Msg::ExecutionReport(er)).await.unwrap();
                            }
                        }
                        _ => (),
                    },
                    _ => (),
                }
            }
        }
    }
}

fn deserialize_books5(v: &Value) -> Depth {
    let inst_id = v["instId"].as_str().unwrap();
    let inst = parser::str_to_inst(inst_id);
    let exch_time = parser::str_to_naive_datetime(v["ts"].as_str().unwrap());
    let recv_time = chrono::Utc::now().naive_utc();
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
    Depth {
        inst,
        exch_time,
        recv_time,
        asks,
        bids,
    }
}

fn deserialize_trades(v: &Value) -> Trade {
    Trade {
        inst: parser::str_to_inst(v["instId"].as_str().unwrap()),
        exch_time: parser::str_to_naive_datetime(v["ts"].as_str().unwrap()),
        recv_time: chrono::Utc::now().naive_utc(),
        side: parser::str_to_side(v["side"].as_str().unwrap()),
        px: v["px"].as_str().unwrap().parse().unwrap(),
        sz: v["sz"].as_str().unwrap().parse().unwrap(),
    }
}

fn deserialize_balance_and_position(v: &Value) -> Vec<BalanceReport> {
    let mut msgs = Vec::new();
    for d in v["balData"].as_array().unwrap() {
        let ccy = d["ccy"].as_str().unwrap().try_into().unwrap_or_default();
        let u_time = parser::str_to_naive_datetime(d["uTime"].as_str().unwrap());
        let cash_bal = d["cashBal"].as_str().unwrap().parse().unwrap();
        let br = BalanceReport {
            u_time,
            exch: Exch::Okx,
            ccy,
            cash_bal,
        };
        msgs.push(br);
    }
    msgs
}

fn deserialize_positions(v: &Value) -> PositionReport {
    PositionReport {
        u_time: parser::str_to_naive_datetime(v["uTime"].as_str().unwrap()),
        inst: parser::str_to_inst(v["instId"].as_str().unwrap()),
        mgn_mode: parser::str_to_mgn_mode(v["mgnMode"].as_str().unwrap()),
        pos: v["pos"].as_str().unwrap().parse().unwrap(),
        ccy: v["ccy"].as_str().unwrap().try_into().unwrap_or_default(),
        pos_ccy: v["posCcy"].as_str().unwrap().try_into().unwrap_or_default(),
        avg_px: v["avgPx"].as_str().unwrap().parse().unwrap(),
    }
}

fn deserialize_orders(v: &Value) -> ExecutionReport {
    ExecutionReport {
        c_time: parser::str_to_naive_datetime(v["cTime"].as_str().unwrap()),
        u_time: parser::str_to_naive_datetime(v["uTime"].as_str().unwrap()),
        inst: parser::str_to_inst(v["instId"].as_str().unwrap()),
        ccy: v["ccy"].as_str().unwrap().try_into().unwrap_or_default(),
        ord_id: v["ordId"].as_str().unwrap().parse().unwrap(),
        cl_ord_id: v["clOrdId"].as_str().unwrap().parse().unwrap_or_default(),
        px: v["px"].as_str().unwrap().parse().unwrap_or_default(),
        sz: v["sz"].as_str().unwrap().parse().unwrap_or_default(),
        notional_usd: v["notionalUsd"]
            .as_str()
            .unwrap()
            .parse()
            .unwrap_or_default(),
        ord_type: parser::str_to_ord_type(v["ordType"].as_str().unwrap()),
        side: parser::str_to_side(v["side"].as_str().unwrap()),
        fill_px: v["fillPx"].as_str().unwrap().parse().unwrap_or(0.0),
        fill_sz: v["fillSz"].as_str().unwrap().parse().unwrap_or(0.0),
        acc_fill_sz: v["accFillSz"].as_str().unwrap().parse().unwrap_or(0.0),
        avg_px: v["avgPx"].as_str().unwrap().parse().unwrap_or(0.0),
        state: parser::str_to_ord_state(v["state"].as_str().unwrap()),
        lever: v["lever"].as_str().unwrap().parse().unwrap(),
        fee: v["fee"].as_str().unwrap().parse().unwrap(),
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn deserialize_login_result_works() {
        let s = r#"
        {
            "event": "login",
            "code": "0",
            "msg": ""
        }"#;
        let m: WsMessage = serde_json::from_str(s).unwrap();
        let WsMessage::LoginResult { code, msg: _ } = m else{
            panic!("deserialize login result failed");
        };
        assert_eq!(code, "0");

        let s = r#"
        {
            "event": "error",
            "code": "60009",
            "msg": "Login failed."
        }"#;
        let m: WsMessage = serde_json::from_str(s).unwrap();
        let WsMessage::LoginResult { code, msg: _ } = m else{
            panic!("deserialize login result failed");
        };
        assert_ne!(code, "0");
    }

    #[test]
    fn deserialize_subscribe_result() {
        let s = r#"
        {
            "event": "subscribe",
            "arg": {
                "channel": "books5",
                "instId": "LTC-USD-200327"
            }
        }"#;
        let m: WsMessage = serde_json::from_str(s).unwrap();
        let WsMessage::SubscribeResult { arg: _} = m else{
            panic!("deserialize subscribe result failed");
        };
    }

    #[test]
    fn deserialize_data_balance_and_position_works() {
        let s = r#"
        {
            "arg": {
                "channel": "balance_and_position",
                "uid": "77982378738415879"
            },
            "data": [{
                "pTime": "1597026383085",
                "eventType": "snapshot",
                "balData": [{
                    "ccy": "BTC",
                    "cashBal": "1",
                    "uTime": "1597026383085"
                }],
                "posData": [{
                    "posId": "1111111111",
                    "tradeId": "2",
                    "instId": "BTC-USD-191018",
                    "instType": "FUTURES",
                    "mgnMode": "cross",
                    "posSide": "long",
                    "pos": "10",
                    "ccy": "BTC",
                    "posCcy": "",
                    "avgPx": "3320",
                    "uTime": "1597026383085"
                }]
            }]
        }"#;
        let m: WsMessage = serde_json::from_str(s).unwrap();
        let WsMessage::Data { arg:_, data } = m else{
            panic!("deserialize balance and position failed");
        };
        let brs = deserialize_balance_and_position(&data[0]);
        assert_eq!(brs[0].cash_bal, 1.0);
        assert_eq!(brs[0].ccy, Ccy::BTC);
    }

    #[test]
    fn deserialize_orders_works() {
        let s = r#"
        {
            "arg": {
                "channel": "orders",
                "instType": "SPOT",
                "instId": "BTC-USDT",
                "uid": "614488474791936"
            },
            "data": [
                {
                    "accFillSz": "0.001",
                    "amendResult": "",
                    "avgPx": "31527.1",
                    "cTime": "1654084334977",
                    "category": "normal",
                    "ccy": "",
                    "clOrdId": "",
                    "code": "0",
                    "execType": "M",
                    "fee": "-0.02522168",
                    "feeCcy": "USDT",
                    "fillFee": "-0.02522168",
                    "fillFeeCcy": "USDT",
                    "fillNotionalUsd": "31.50818374",
                    "fillPx": "31527.1",
                    "fillSz": "0.001",
                    "fillTime": "1654084353263",
                    "instId": "BTC-USDT",
                    "instType": "SPOT",
                    "lever": "0",
                    "msg": "",
                    "notionalUsd": "31.50818374",
                    "ordId": "452197707845865472",
                    "ordType": "limit",
                    "pnl": "0",
                    "posSide": "",
                    "px": "31527.1",
                    "rebate": "0",
                    "rebateCcy": "BTC",
                    "reduceOnly": "false",
                    "reqId": "",
                    "side": "sell",
                    "slOrdPx": "",
                    "slTriggerPx": "",
                    "slTriggerPxType": "last",
                    "source": "",
                    "state": "filled",
                    "sz": "0.001",
                    "tag": "",
                    "tdMode": "cash",
                    "tgtCcy": "",
                    "tpOrdPx": "",
                    "tpTriggerPx": "",
                    "tpTriggerPxType": "last",
                    "tradeId": "242589207",
                    "quickMgnType": "",
                    "algoClOrdId": "",
                    "algoId": "",
                    "amendSource": "",
                    "cancelSource": "",
                    "uTime": "1654084353264"
                }
            ]
        }"#;
        let m: WsMessage = serde_json::from_str(s).unwrap();
        let WsMessage::Data { arg:_, data } = m else{
            panic!("deserialize orders failed");
        };
        let er = deserialize_orders(&data[0]);
        assert_eq!(er.ord_id, 452197707845865472);
        assert_eq!(er.state, OrdState::Filled);
        assert_eq!(er.notional_usd, 31.50818374);
        assert_eq!(er.ord_type, OrdType::Limit);
    }

    #[test]
    fn deserialize_positions_works() {
        let s = r#"
        {
            "arg":{
                "channel":"positions",
                "uid": "77982378738415879",
                "instType":"FUTURES"
            },
            "data":[
                {
                    "adl":"1",
                    "availPos":"1",
                    "avgPx":"2566.31",
                    "cTime":"1619507758793",
                    "ccy":"ETH",
                    "deltaBS":"",
                    "deltaPA":"",
                    "gammaBS":"",
                    "gammaPA":"",
                    "imr":"",
                    "instId":"ETH-USD-210430",
                    "instType":"FUTURES",
                    "interest":"0",
                    "last":"2566.22",
                    "lever":"10",
                    "liab":"",
                    "liabCcy":"",
                    "liqPx":"2352.8496681818233",
                    "markPx":"2353.849",
                    "margin":"0.0003896645377994",
                    "mgnMode":"isolated",
                    "mgnRatio":"11.731726509588816",
                    "mmr":"0.0000311811092368",
                    "notionalUsd":"2276.2546609009605",
                    "optVal":"",
                    "pTime":"1619507761462",
                    "pos":"1",
                    "baseBorrowed": "",
                    "baseInterest": "",
                    "quoteBorrowed": "",
                    "quoteInterest": "",
                    "posCcy":"",
                    "posId":"307173036051017730",
                    "posSide":"long",
                    "spotInUseAmt": "",
                    "spotInUseCcy": "",
                    "bizRefId": "",
                    "bizRefType": "",
                    "thetaBS":"",
                    "thetaPA":"",
                    "tradeId":"109844",
                    "uTime":"1619507761462",
                    "upl":"-0.0000009932766034",
                    "uplLastPx":"-0.0000009932766034",
                    "uplRatio":"-0.0025490556801078",
                    "uplRatioLastPx":"-0.0025490556801078",
                    "vegaBS":"",
                    "vegaPA":"",
                    "closeOrderAlgo":[
                        {
                            "algoId":"123",
                            "slTriggerPx":"123",
                            "slTriggerPxType":"mark",
                            "tpTriggerPx":"123",
                            "tpTriggerPxType":"mark",
                            "closeFraction":"0.6"
                        },
                        {
                            "algoId":"123",
                            "slTriggerPx":"123",
                            "slTriggerPxType":"mark",
                            "tpTriggerPx":"123",
                            "tpTriggerPxType":"mark",
                            "closeFraction":"0.4"
                        }
                    ]
                }
            ]
        }"#;
        let m: WsMessage = serde_json::from_str(s).unwrap();
        let WsMessage::Data { arg:_, data } = m else{
            panic!("deserialize positions failed");
        };
        let pr = deserialize_positions(&data[0]);
        assert_eq!(pr.mgn_mode, MgnMode::Isolated);
        assert_eq!(pr.pos, 1.0);
    }

    #[test]
    fn deserialize_books5_works() {
        let s = r#"{
            "arg": {
                "channel": "books5",
                "instId": "BCH-USDT-SWAP"
            },
            "data": [{
                "asks": [
                    ["111.06","55154","0","2"],
                    ["111.07","53276","0","2"],
                    ["111.08","72435","0","2"],
                    ["111.09","70312","0","2"],
                    ["111.1","67272","0","2"]],
                "bids": [
                    ["111.05","57745","0","2"],
                    ["111.04","57109","0","2"],
                    ["111.03","69563","0","2"],
                    ["111.02","71248","0","2"],
                    ["111.01","65090","0","2"]],
                "instId": "BCH-USDT-SWAP",
                "ts": "1670324386802"
            }]
        }"#;
        let msg: WsMessage = serde_json::from_str(s).unwrap();
        let WsMessage::Data{ arg:_, data } = msg else{
            panic!("deserialize books5 failed");
        };
        let books = deserialize_books5(&data[0]);
        assert_eq!(books.asks[0].0, 111.06);
        assert_eq!(books.asks[0].1, 55154.0);
        assert_eq!(books.bids[4].0, 111.01);
        assert_eq!(books.bids[4].1, 65090.0);
    }

    #[test]
    fn deserialize_trades_works() {
        let s = r#"
        {
            "arg": {
              "channel": "trades",
              "instId": "BTC-USDT"
            },
            "data": [
              {
                "instId": "BTC-USDT",
                "tradeId": "130639474",
                "px": "42219.9",
                "sz": "0.12060306",
                "side": "buy",
                "ts": "1630048897897"
              }
            ]
        }"#;
        let msg: WsMessage = serde_json::from_str(s).unwrap();
        let WsMessage::Data{ arg:_, data } = msg else{
            panic!("deserialize trades failed");
        };
        let trade = deserialize_trades(&data[0]);
        assert_eq!(trade.side, Side::Buy);
        assert_eq!(trade.px, 42219.9);
        assert_eq!(trade.sz, 0.12060306);
    }
}
