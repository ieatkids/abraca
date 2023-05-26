use crate::common::{
    defs::{DataType, Inst, Result},
    msgs::{MsgReceiver, MsgSender},
    traits::Api,
};
use ws::{PrivateClient, PublicClient, WsChannel, WsChannelArg};

pub(self) mod parser;
pub(self) mod ws;

pub struct OkxApi {
    public_client: ws::PublicClient,
    private_client: Option<ws::PrivateClient>,
}

impl OkxApi {
    pub fn builder() -> WsClientBuilder {
        WsClientBuilder::new()
    }
}

impl Api for OkxApi {
    fn name(&self) -> &'static str {
        "OkxApi"
    }

    async fn start(self, tx: MsgSender, rx: MsgReceiver) -> Result<()> {
        log::info!("start okx websocket client");
        if let Some(private_client) = self.private_client {
            log::info!("start okx private websocket client");
            let tx = tx.clone();
            tokio::spawn(async move {
                private_client
                    .start(tx, rx)
                    .await
                    .expect("start okx private websocket client error");
            });
        }
        log::info!("start okx public websocket client");
        self.public_client.start(tx).await
    }
}

#[derive(Default)]
pub struct WsClientBuilder {
    apikey: String,
    secretkey: String,
    passphrase: String,
    channels: Vec<WsChannelArg>,
}

impl WsClientBuilder {
    pub fn new() -> Self {
        Self::default()
    }

    pub fn build(self) -> OkxApi {
        if self.apikey.is_empty() || self.secretkey.is_empty() || self.passphrase.is_empty() {
            OkxApi {
                public_client: PublicClient {
                    channels: self.channels,
                },
                private_client: None,
            }
        } else {
            let private_channels = vec![
                WsChannelArg {
                    channel: WsChannel::BalanceAndPosition,
                    inst_id: None,
                    inst_type: None,
                    inst_family: None,
                },
                WsChannelArg {
                    channel: WsChannel::Orders,
                    inst_id: None,
                    inst_type: Some("ANY".to_string()),
                    inst_family: None,
                },
                WsChannelArg {
                    channel: WsChannel::Positions,
                    inst_id: None,
                    inst_type: Some("ANY".to_string()),
                    inst_family: None,
                },
            ];
            OkxApi {
                public_client: PublicClient {
                    channels: self.channels,
                },
                private_client: Some(PrivateClient {
                    apikey: self.apikey,
                    secretkey: self.secretkey,
                    passphrase: self.passphrase,
                    channels: private_channels,
                }),
            }
        }
    }

    pub fn credential(mut self, apikey: String, secretkey: String, passphrase: String) -> Self {
        self.apikey = apikey;
        self.secretkey = secretkey;
        self.passphrase = passphrase;
        self
    }

    pub fn subscribe<S, I, D>(mut self, subs: S) -> Self
    where
        I: TryInto<Inst>,
        D: TryInto<DataType>,
        S: IntoIterator<Item = (I, D)>,
    {
        for (i, d) in subs {
            if let Ok(inst) = i.try_into() {
                if let Ok(data_type) = d.try_into() {
                    let channel = match data_type {
                        DataType::Depth => WsChannel::Books5,
                        DataType::Trade => WsChannel::Trade,
                        DataType::Ticker => WsChannel::Tickers,
                        DataType::OpenInterest => WsChannel::OpenInterest,
                        DataType::FundingRate => WsChannel::FundingRate,
                    };
                    self.channels.push(WsChannelArg {
                        channel,
                        inst_id: Some(parser::inst_to_str(&inst)),
                        inst_type: None,
                        inst_family: None,
                    });
                }
            }
        }
        self
    }
}
