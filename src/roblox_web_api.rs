use std::{fmt, fs, path::Path};

use reqwest::{
    header::{HeaderValue, COOKIE},
    multipart::{Form, Part},
    Client, Request, Response, StatusCode,
};
use serde::{Deserialize, Serialize};

#[derive(Debug, Clone)]
pub struct ImageUploadData<'a> {
    pub image_data: Vec<u8>,
    pub name: &'a str,
    pub description: &'a str,
}

#[derive(Debug, Serialize, Deserialize)]
#[serde(rename_all = "PascalCase")]
pub struct UploadResponse {
    pub success: bool,
    pub asset_id: u64,
    pub backing_asset_id: u64,
}

pub struct RobloxApiClient {
    auth_token: String,
    csrf_token: Option<HeaderValue>,
    client: Client,
}

impl fmt::Debug for RobloxApiClient {
    fn fmt(&self, formatter: &mut fmt::Formatter) -> fmt::Result {
        write!(formatter, "<RobloxApiClient>")
    }
}

impl RobloxApiClient {
    pub fn new(auth_token: String) -> Self {
        Self {
            auth_token,
            csrf_token: None,
            client: Client::new(),
        }
    }

    pub fn upload_image(&mut self, data: ImageUploadData) -> reqwest::Result<UploadResponse> {
        let url = "https://data.roblox.com/data/upload/json?assetTypeId=13";

        let mut response = self.execute_with_csrf_retry(|client| {
            client
                .post(url)
                .query(&[("name", data.name), ("description", data.description)])
                .body(data.image_data.clone())
                .build()
        })?;

        if response.status().is_success() {
            let body: UploadResponse = match response.json() {
                Ok(body) => body,
                Err(err) => {
                    panic!("Got malformed API response: {}", err);
                }
            };

            Ok(body)
        } else {
            let body = response.text().unwrap();

            log::error!("response: {:?}", response);
            log::error!("status: {:?}", response.status());
            log::error!("body: {}", body);

            unimplemented!("Handle bad responses");
        }
    }

    fn execute_with_csrf_retry<F>(&mut self, make_request: F) -> reqwest::Result<Response>
    where
        F: Fn(&Client) -> reqwest::Result<Request>,
    {
        let mut request = make_request(&self.client)?;
        self.attach_headers(&mut request);

        let response = self.client.execute(request)?;

        match response.status() {
            StatusCode::FORBIDDEN => {
                if let Some(csrf) = response.headers().get("X-CSRF-Token") {
                    log::debug!("Retrying request with X-CSRF-Token...");

                    self.csrf_token = Some(csrf.clone());

                    let mut new_request = make_request(&self.client)?;
                    self.attach_headers(&mut new_request);

                    self.client.execute(new_request)
                } else {
                    Ok(response)
                }
            }
            _ => Ok(response),
        }
    }

    fn attach_headers(&self, request: &mut Request) {
        let cookie_value = format!(".ROBLOSECURITY={}", self.auth_token);

        request.headers_mut().insert(
            COOKIE,
            HeaderValue::from_bytes(cookie_value.as_bytes()).unwrap(),
        );

        if let Some(csrf) = &self.csrf_token {
            request.headers_mut().insert("X-CSRF-Token", csrf.clone());
        }
    }

    /// I think this method is supposed to work, but currently does not upload
    /// assets of type Image correctly.
    // TODO: Switch to using this endpoint instead if it can work for us.
    #[allow(dead_code)]
    fn upload_image_publish_api(&mut self, path: &Path) {
        let url = "https://publish.roblox.com/v1/assets/upload";

        let mut response = self
            .execute_with_csrf_retry(|client| {
                let config = Part::text(
                    r#"
                {
                  "apple": {
                    "description": "I tried.",
                    "name": "Apple",
                    "type": "Image"
                  }
                }"#,
                )
                .file_name("config.json")
                .mime_str("application/json")?;

                let buffer = fs::read(path).expect("Couldn't read file");
                let apple = Part::bytes(buffer)
                    .file_name("apple.png")
                    .mime_str("image/png")?;

                let form = Form::new().part("config", config).part("apple", apple);

                client.post(url).multipart(form).build()
            })
            .unwrap();

        let body = response.text().unwrap();

        println!("response: {:?}", response);
        println!("status: {:?}", response.status());
        println!("body: {}", body);
    }
}
