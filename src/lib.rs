use secstr::SecUtf8;
use serde::{Deserialize, Serialize};

#[derive(Deserialize, Debug, Serialize)]
pub struct Message {
    pub secret: SecUtf8,
    #[serde(flatten)]
    pub request: MessageRequest,
}

#[derive(Deserialize, Debug, Serialize)]
#[serde(tag = "type")]
#[serde(rename_all = "camelCase")]
pub enum MessageRequest {
    Terminal { path: String },
    Code { path: String },
}
