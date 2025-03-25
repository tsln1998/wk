use serde::Deserialize;
use serde::Serialize;

#[derive(Serialize, Deserialize, Clone, Debug)]
pub struct CaptchaGenerateReq {
    pub w: Option<u32>,
    pub h: Option<u32>,
}

#[derive(Serialize, Deserialize, Clone, Debug)]
pub struct CaptchaGenerateResp {
    pub id: String,
    pub base64: String,
}
