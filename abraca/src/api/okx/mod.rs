use crate::prelude::*;
use base64::{engine::general_purpose, Engine};
use ring::hmac;

mod parser;
mod rest;
mod ws;

fn get_sign(ts: &str, method: &str, path: &str, body: &str, secretkey: &str) -> String {
    let key = hmac::Key::new(hmac::HMAC_SHA256, secretkey.as_bytes());
    let sign = hmac::sign(&key, format!("{ts}{method}{path}{body}").as_bytes());
    general_purpose::STANDARD.encode(sign)
}

#[derive(Clone)]
pub struct OkxCredential {
    apikey: String,
    secretkey: String,
    passphrase: String,
}

pub struct OkxApi {
    credential: Option<OkxCredential>,
    subs: Vec<Inst>,
}

impl OkxApi {
    pub fn builder() -> OkxApiBuilder {
        OkxApiBuilder::default()
    }

    pub async fn start(self, tx: MsgSender, rx: MsgReceiver) {
        if let Some(credential) = self.credential {
            log::info!("start okx order gateway");
            let t1 = tx.clone();
            let t2 = tx.clone();
            let ws = ws::PrivateWsClient::new(&self.subs, &credential);
            let rest = rest::RestClient::new(&credential);
            tokio::spawn(async move {
                ws.run(t1).await;
            });
            tokio::spawn(async move {
                rest.run(t2, rx).await;
            });
        }
        log::info!("start okx market gateway");
        let ws = ws::PublicWsClient::new(&self.subs);
        tokio::spawn(async move { ws.run(tx).await });
    }
}

#[derive(Default)]
pub struct OkxApiBuilder {
    credential: Option<OkxCredential>,
    subs: Vec<Inst>,
}

impl OkxApiBuilder {
    pub fn new() -> Self {
        Self::default()
    }

    pub fn build(self) -> OkxApi {
        OkxApi {
            credential: self.credential,
            subs: self.subs,
        }
    }

    pub fn with_credential(mut self, apikey: &str, secretkey: &str, passphrase: &str) -> Self {
        self.credential = Some(OkxCredential {
            apikey: apikey.to_owned(),
            secretkey: secretkey.to_owned(),
            passphrase: passphrase.to_owned(),
        });
        self
    }

    pub fn subscribe<InstIter, C, I>(mut self, insts: InstIter) -> Self
    where
        C: TryInto<Ccy>,
        I: TryInto<InstType>,
        InstIter: IntoIterator<Item = (C, C, I)>,
    {
        self.subs
            .extend(insts.into_iter().map(|(base_ccy, quote_ccy, inst_type)| {
                Inst::try_from((Exch::Okx, base_ccy, quote_ccy, inst_type)).unwrap()
            }));
        self
    }
}
