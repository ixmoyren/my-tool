use crate::{
    Error::Api, IoOperationSnafu, RequestOperationSnafu, Result, auth::Session,
    crypto::weapi_encrypt,
};
use reqwest::blocking::Client as HttpClient;
use serde_json::Value;
use snafu::ResultExt;
use std::{
    fs::File,
    io::{BufWriter, Write},
    path::Path,
};

const BASE_URL: &str = "https://music.163.com";
const USER_AGENT: &str = "Mozilla/5.0 (Macintosh; Intel Mac OS X 10_15_7) \
    AppleWebKit/537.36 (KHTML, like Gecko) Chrome/124.0.0.0 Safari/537.36";

pub struct Client {
    http: HttpClient,
    session: Session,
}

impl Client {
    pub fn new() -> Result<Self> {
        let http = Self::http_client()?;
        let session = Session::load()?;
        Ok(Self { http, session })
    }

    pub fn with_session(session: Session) -> Result<Self> {
        let http = Self::http_client()?;
        Ok(Self { http, session })
    }
    pub fn session(&self) -> &Session {
        &self.session
    }

    pub fn request(&self, endpoint: &str, data: &Value) -> Result<Value> {
        let payload = weapi_encrypt(&data.to_string());
        let url = format!("{BASE_URL}/weapi{endpoint}");

        let mut req = self
            .http
            .post(&url)
            .header("Referer", BASE_URL)
            .header("Content-Type", "application/x-www-form-urlencoded");

        if let Some(cookie) = self.session.cookie_header() {
            req = req.header("Cookie", cookie);
        }

        let body = format!(
            "params={}&encSecKey={}",
            urlencoding::encode(&payload.params),
            payload.enc_sec_key,
        );

        let resp = req.body(body).send().context(RequestOperationSnafu {
            message: format!("Failed to send request to ({url})"),
        })?;
        let json: Value = resp.json().context(RequestOperationSnafu {
            message: format!("Failed to read from response ({url})"),
        })?;

        if let Some(code) = json.get("code").and_then(Value::as_i64)
            && code != 200
        {
            let message = json
                .get("message")
                .or_else(|| json.get("msg"))
                .and_then(Value::as_str)
                .unwrap_or("unknown error")
                .to_owned();
            return Err(Api { code, message });
        }

        Ok(json)
    }

    pub fn download(&self, url: &str, dest: &Path) -> Result<u64> {
        let resp = self
            .http
            .get(url)
            .header("Referer", BASE_URL)
            .send()
            .context(RequestOperationSnafu {
                message: format!("Failed to send request to ({url}) when downloading file",),
            })?;

        let file = File::create(dest).context(IoOperationSnafu {
            message: format!("Failed to create the file({})", dest.display()),
        })?;
        let mut buf = BufWriter::new(file);
        let bytes = resp.bytes().context(RequestOperationSnafu {
            message: "Failed to read the bytes from response".to_owned(),
        })?;
        buf.write_all(&bytes).context(IoOperationSnafu {
            message: format!("Failed to write the file({})", dest.display()),
        })?;
        Ok(bytes.len() as u64)
    }

    fn http_client() -> Result<HttpClient> {
        let retries = reqwest::retry::for_host(BASE_URL)
            .classify_fn(|req_rep| {
                if req_rep.status() == Some(http::StatusCode::SERVICE_UNAVAILABLE) {
                    req_rep.retryable()
                } else {
                    req_rep.success()
                }
            })
            .max_retries_per_request(3);
        HttpClient::builder()
            .user_agent(USER_AGENT)
            .timeout(std::time::Duration::from_secs(30))
            .retry(retries)
            .build()
            .context(RequestOperationSnafu {
                message: "Failed to create the reqwest client".to_owned(),
            })
    }
}
