use super::{get_sign, parser, OkxCredential};
use crate::prelude::*;
use anyhow::anyhow;
use reqwest::header::{HeaderMap, HeaderValue};
use serde_json::{json, Value};

#[cfg(feature = "testnet")]
const REST_URL: &str = "https://www.okx.com";

#[cfg(not(feature = "testnet"))]
const REST_URL: &str = "https://www.okx.com";

pub(super) struct RestClient {
    apikey: String,
    secretkey: String,
    passphrase: String,
    client: reqwest::Client,
}

impl RestClient {
    pub fn new(credential: &OkxCredential) -> Self {
        Self {
            apikey: credential.apikey.to_owned(),
            secretkey: credential.secretkey.to_owned(),
            passphrase: credential.passphrase.to_owned(),
            client: reqwest::Client::new(),
        }
    }

    pub async fn run(self, tx: MsgSender, mut rx: MsgReceiver) {
        while let Some(msg) = rx.recv().await {
            match msg {
                Msg::NewOrder(req) => {
                    log::info!("send order {req:?}");
                    if let Err(msg) = self.send_order(&req).await {
                        log::error!("send order error {}", msg);
                        let ts = chrono::Utc::now().naive_utc();
                        let er = ExecutionReport {
                            c_time: ts,
                            u_time: ts,
                            inst: req.inst,
                            ccy: Ccy::default(),
                            ord_id: 0,
                            cl_ord_id: req.cl_ord_id,
                            px: req.px,
                            sz: req.sz,
                            notional_usd: 0.0,
                            ord_type: req.ord_type,
                            side: req.side,
                            fill_px: 0.0,
                            fill_sz: 0.0,
                            acc_fill_sz: 0.0,
                            avg_px: 0.0,
                            state: OrdState::Rejected,
                            lever: 0.0,
                            fee: 0.0,
                        };
                        tx.send(Msg::ExecutionReport(er)).await.unwrap();
                    }
                }
                Msg::CancelOrder(req) => {
                    log::info!("cancel order {req:?}");
                    if let Err(msg) = self.cancel_order(&req).await {
                        log::error!("cancel order error {}", msg);
                        let cj = CancelReject {
                            u_time: chrono::Utc::now().naive_utc(),
                            inst: req.inst,
                            cl_ord_id: req.cl_ord_id,
                        };
                        tx.send(Msg::CancelReject(cj)).await.unwrap();
                    }
                }
                _ => (),
            }
        }
    }
    
    fn get_headers(&self, path: &str, body: &str) -> Result<HeaderMap> {
        let ts = chrono::Utc::now().to_rfc3339_opts(chrono::SecondsFormat::Millis, true);
        let sign = get_sign(&ts, "POST", path, body, &self.secretkey);
        let mut headers = HeaderMap::new();
        headers.insert("Content-Type", HeaderValue::from_static("application/json"));
        headers.insert("OK-ACCESS-KEY", HeaderValue::from_str(&self.apikey)?);
        headers.insert("OK-ACCESS-SIGN", HeaderValue::from_str(&sign)?);
        headers.insert("OK-ACCESS-TIMESTAMP", HeaderValue::from_str(&ts)?);
        headers.insert(
            "OK-ACCESS-PASSPHRASE",
            HeaderValue::from_str(&self.passphrase)?,
        );
        #[cfg(feature = "testnet")]
        headers.insert("x-simulated-trading", HeaderValue::from_static("1"));
        Ok(headers)
    }

    async fn post(&self, path: &str, body: String) -> Result<()> {
        let headers = self.get_headers(path, &body)?;
        let resp = self
            .client
            .post(format!("{}{}", REST_URL, path))
            .headers(headers)
            .body(body)
            .send()
            .await?
            .text()
            .await?;
        let v: Value = serde_json::from_str(&resp).unwrap();
        let code = v["code"].as_str().unwrap();
        if code != "0" {
            let msg = v["msg"].as_str().unwrap();
            return Err(anyhow!(msg.to_owned()));
        }
        let s_code = v["data"][0]["sCode"].as_str().unwrap();
        if s_code == "0" {
            Ok(())
        } else {
            let s_msg = v["data"][0]["sMsg"].as_str().unwrap();
            Err(anyhow!(s_msg.to_owned()))
        }
    }

    async fn send_order(&self, no: &NewOrder) -> Result<()> {
        let req = json!({
            "instId": parser::inst_to_str(&no.inst),
            "tdMode": parser::td_mod_to_str(&no.td_mod),
            "side": parser::side_to_str(&no.side),
            "ordType": parser::ord_type_to_str(&no.ord_type),
            "px": no.px.to_string(),
            "sz": no.sz.to_string(),
        });
        let body = serde_json::to_string(&req)?;
        self.post("/api/v5/trade/order", body).await
    }

    async fn cancel_order(&self, co: &CancelOrder) -> Result<()> {
        let req = json!({
            "clOrdId": co.cl_ord_id,
            "instId": parser::inst_to_str(&co.inst),
        });
        let body = serde_json::to_string(&req)?;
        self.post("/api/v5/trade/order", body).await
    }
}
