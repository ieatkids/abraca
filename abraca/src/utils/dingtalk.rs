use crate::common::defs::Result;
use anyhow::anyhow;
use reqwest::{header::HeaderMap, Client, ClientBuilder};
use serde::Deserialize;
use serde_json::{json, Value};

/// `DingTalk` is a struct for sending message to DingTalk.
pub struct DingTalk {
    keyword: String,
    webhook: String,
    client: Client,
}

#[derive(Debug, Deserialize)]
struct DingTalkResp {
    errcode: i32,
    errmsg: String,
}

impl DingTalk {
    pub fn new(keyword: &str, access_token: &str) -> Self {
        let mut headers = HeaderMap::new();
        headers.insert("Content-Type", "application/json".parse().unwrap());
        Self {
            keyword: keyword.to_owned(),
            webhook: format!(
                "https://oapi.dingtalk.com/robot/send?access_token={}",
                access_token
            ),
            client: ClientBuilder::new()
                .default_headers(headers)
                .build()
                .unwrap(),
        }
    }

    fn text_params(&self, s: &str) -> Value {
        json!({
                "msgtype": "text",
                "text": {
                    "content": format!("{}:{}\n",self.keyword, s)
            }
        })
    }

    fn markdown_params(&self, s: &str) -> Value {
        json!({
            "msgtype": "markdown",
            "markdown": {
                "title": self.keyword,
                "text": s
            }
        })
    }

    pub async fn send_msg(&self, content: &str, is_markdown: bool) -> Result<()> {
        let params = if is_markdown {
            self.markdown_params(content)
        } else {
            self.text_params(content)
        };
        let res: DingTalkResp = self
            .client
            .post(&self.webhook)
            .json(&params)
            .send()
            .await?
            .error_for_status()?
            .json()
            .await?;
        if res.errcode == 0 {
            Ok(())
        } else {
            Err(anyhow!("DingTalk error: {}", res.errmsg))
        }
    }

    /// `send_text` sends a text message to DingTalk.
    /// # Arguments
    /// * `s` - The text message.
    pub async fn send_text(&self, s: &str) -> Result<()> {
        self.send_msg(s, false).await
    }

    /// `send_markdown` sends a markdown message to DingTalk.
    /// # Arguments
    /// * `s` - The markdown message.
    pub async fn send_markdown(&self, s: &str) -> Result<()> {
        self.send_msg(s, true).await
    }
}
