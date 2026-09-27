use reqwest::Client as HttpClient;

pub(crate) const BASE_URL: &str = "https://api-t1.fyers.in/api/v3";

pub struct FyersClient {
    pub(crate) http: HttpClient,
    pub(crate) app_id: String,
    pub(crate) secret_key: String,
    pub(crate) access_token: Option<String>,
}

impl FyersClient {
    pub fn new(app_id: &str, secret_key: &str) -> Self {
        Self {
            http: HttpClient::new(),
            app_id: app_id.to_string(),
            secret_key: secret_key.to_string(),
            access_token: None,
        }
    }

    pub fn set_token(&mut self, token: String) {
        self.access_token = Some(token);
    }

    pub(crate) fn get_app_id_hash(&self) -> String {
        use sha2::{Digest, Sha256};
        let payload = format!("{}:{}", self.app_id, self.secret_key);
        let mut hasher = Sha256::new();
        hasher.update(payload.as_bytes());
        hex::encode(hasher.finalize())
    }
}
