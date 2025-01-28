use crate::auth_cookie::get_auth_cookie;
use crate::roblox_web_api_types::{
    ImageUploadData, ImageUploadMetadata, RawOperationStatusResponse,
    RawOperationStatusResponseVariants, RawUploadResponse, RobloxAuthenticationError,
    UploadResponse,
};
use log;
use reqwest::{
    header::{HeaderValue, COOKIE},
    multipart, Client, Request, Response, StatusCode,
};

use std::{
    fmt::{self},
    time::Duration,
};
use thiserror::Error;

const OPEN_CLOUD_ASSET_UPLOAD_USER_AUTH: &str =
    "https://apis.roblox.com/assets/user-auth/v1/assets";
const OPEN_CLOUD_ASSET_UPLOAD: &str = "https://apis.roblox.com/assets/v1/assets";

const OPEN_CLOUD_ASSET_OPERATIONS_USER_AUTH: &str =
    "https://apis.roblox.com/assets/user-auth/v1/operations";
const OPEN_CLOUD_ASSET_OPERATIONS: &str = "https://apis.roblox.com/assets/v1/operations";

const OPEN_CLOUD_API_KEY_HEADER: &str = "X-API-Key";
pub const DECAL: &str = "Decal";

pub struct RobloxOpenCloudCredentials {
    auth: RobloxOpenCloudAuth,
}

enum RobloxOpenCloudAuth {
    Cookie(String),
    ApiKey(String),
    None,
}

impl RobloxOpenCloudCredentials {
    pub fn get_credentials(
        cookie: Option<String>,
        api_key: Option<String>,
    ) -> Result<Self, RobloxAuthenticationError> {
        let auth = match (cookie, api_key) {
            (Some(_), Some(_)) => Err(RobloxAuthenticationError::InvalidAuthProvided),
            (Some(cookie), None) => Ok(RobloxOpenCloudAuth::Cookie(cookie)),
            (None, Some(api_key)) => Ok(RobloxOpenCloudAuth::ApiKey(api_key)),
            (None, None) => {
                log::debug!("No authentication provided, attempting to get cookie...");

                if let Some(cookie) = get_auth_cookie() {
                    log::debug!("Cookie found");
                    Ok(RobloxOpenCloudAuth::Cookie(cookie))
                } else {
                    log::debug!("No authentication provided, and failed to get cookie");
                    Ok(RobloxOpenCloudAuth::None)
                }
            }
        }?;

        Ok(Self { auth })
    }
}

pub struct RobloxApiClient {
    credentials: RobloxOpenCloudCredentials,
    csrf_token: Option<HeaderValue>,
    client: Client,
}

impl fmt::Debug for RobloxApiClient {
    fn fmt(&self, formatter: &mut fmt::Formatter) -> fmt::Result {
        write!(formatter, "RobloxApiClient")
    }
}

impl RobloxApiClient {
    pub fn new(credentials: RobloxOpenCloudCredentials) -> Self {
        Self {
            credentials,
            csrf_token: None,
            client: Client::new(),
        }
    }

    pub fn download_image(&mut self, id: u64) -> Result<Vec<u8>, RobloxApiError> {
        let url = format!("https://roblox.com/asset?id={}", id);

        let mut response =
            self.execute_with_csrf_retry(|client| Ok(client.get(&url).build()?))?;

        let mut buffer = Vec::new();
        response.copy_to(&mut buffer)?;

        Ok(buffer)
    }

    /// Upload an image, retrying if the asset endpoint determines that the
    /// asset's name is inappropriate. The asset's name will be replaced with a
    /// generic known-good string.
    pub fn upload_image_with_moderation_retry(
        &mut self,
        data: ImageUploadData,
    ) -> Result<UploadResponse, RobloxApiError> {
        let response = self.upload_image_raw(&data)?;

        match response {
            RawUploadResponse::Success { operation_id, .. } => {
                let asset_id = self.poll_operation_until_complete(operation_id.as_str())?;
                Ok(UploadResponse {
                    asset_id: asset_id.parse::<u64>().unwrap(),
                })
            }
            RawUploadResponse::Error { code: _, message } => {
                if message.contains("fully moderated") {
                    log::warn!(
                        "Image name '{}' was moderated, retrying with different name...",
                        data.image_metadata.display_name
                    );

                    let new_data = ImageUploadData {
                        image_data: data.image_data,
                        image_metadata: ImageUploadMetadata {
                            display_name: "image".to_owned(),
                            ..data.image_metadata
                        },
                    };
                    self.upload_image(new_data)
                } else {
                    Err(RobloxApiError::ApiError { message })
                }
            }
        }
    }

    /// Upload an image, returning an error if anything goes wrong.
    pub fn upload_image(
        &mut self,
        data: ImageUploadData,
    ) -> Result<UploadResponse, RobloxApiError> {
        let response = self.upload_image_raw(&data)?;

        match response {
            RawUploadResponse::Success {
                path: _,
                operation_id,
                done: _,
            } => {
                let asset_id = self.poll_operation_until_complete(operation_id.as_str())?;
                Ok(UploadResponse {
                    asset_id: asset_id.parse::<u64>().unwrap(),
                })
            }
            RawUploadResponse::Error { code: _, message } => {
                Err(RobloxApiError::ApiError { message })
            }
        }
    }

    /// Upload an image, returning the raw response returned by the endpoint,
    /// which may have further failures to handle.
    fn upload_image_raw(
        &mut self,
        data: &ImageUploadData,
    ) -> Result<RawUploadResponse, RobloxApiError> {
        let url = match self.credentials.auth {
            RobloxOpenCloudAuth::Cookie(_) => OPEN_CLOUD_ASSET_UPLOAD_USER_AUTH,
            RobloxOpenCloudAuth::ApiKey(_) => OPEN_CLOUD_ASSET_UPLOAD,
            RobloxOpenCloudAuth::None => {
                return Err(RobloxApiError::ApiError {
                    message: "No authentication provided".to_string(),
                })
            }
        };

        let mut response = self.execute_with_csrf_retry(|client| {
            let metadata = serde_json::to_string(&data.image_metadata).unwrap();

            let form = multipart::Form::new().text("request", metadata).part(
                "fileContent",
                multipart::Part::bytes(data.image_data.clone().into_owned()).file_name("image"),
            );
            let request = client.post(url).multipart(form).build()?;
            Ok(request)
        })?;

        let body = response.text()?;

        // Some errors will be reported through HTTP status codes, handled here.
        if response.status().is_success() {
            match serde_json::from_str(&body) {
                Ok(response) => Ok(response),
                Err(source) => Err(RobloxApiError::BadResponseJson { body, source }),
            }
        } else {
            Err(RobloxApiError::ResponseError {
                status: response.status(),
                body,
            })
        }
    }

    /// Execute a request generated by the given function, retrying if the
    /// endpoint requests that the user refreshes their CSRF token.
    fn execute_with_csrf_retry<F>(&mut self, make_request: F) -> Result<Response, RobloxApiError>
    where
        F: Fn(&Client) -> Result<Request, RobloxApiError>,
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
                    Ok(self.client.execute(new_request)?)
                } else {
                    // If the response did not return a CSRF token for us to
                    // retry with, this request was likely forbidden for other
                    // reasons.

                    Ok(response)
                }
            }
            _ => Ok(response),
        }
    }

    /// Attach required headers to a request object before sending it to a
    /// Roblox API, like authentication and CSRF protection.
    fn attach_headers(&self, request: &mut Request) {
        let credentials = &self.credentials;

        match &credentials.auth {
            RobloxOpenCloudAuth::Cookie(cookie) => {
                let cookie_value = format!(".ROBLOSECURITY={}", cookie);

                request.headers_mut().insert(
                    COOKIE,
                    HeaderValue::from_bytes(cookie_value.as_bytes()).unwrap(),
                );
            }
            RobloxOpenCloudAuth::ApiKey(api_key) => {
                request.headers_mut().insert(
                    OPEN_CLOUD_API_KEY_HEADER,
                    HeaderValue::from_bytes(api_key.as_bytes()).unwrap(),
                );
            }
            RobloxOpenCloudAuth::None => {}
        };

        if let Some(csrf) = &self.csrf_token {
            request.headers_mut().insert("X-CSRF-Token", csrf.clone());
        }
    }

    fn poll_operation_until_complete(
        &mut self,
        operation_id: &str,
    ) -> Result<String, RobloxApiError> {
        let base_url = match self.credentials.auth {
            RobloxOpenCloudAuth::Cookie(_) => OPEN_CLOUD_ASSET_OPERATIONS_USER_AUTH,
            RobloxOpenCloudAuth::ApiKey(_) => OPEN_CLOUD_ASSET_OPERATIONS,
            RobloxOpenCloudAuth::None => {
                return Err(RobloxApiError::ApiError {
                    message: "No authentication provided".to_string(),
                })
            }
        };

        let url = format!("{}/{}", base_url, operation_id);
        const FIRST_TRY: u32 = 1;
        const MAX_RETRIES: u32 = 5;
        const BASE_DELAY: Duration = Duration::from_millis(2000);
        const STEP_DELAY: Duration = Duration::from_millis(50);
        const EXPONENTIAL_BACKOFF: u32 = 2;
        log::debug!("Polling operation until complete: {}", operation_id);
        for attempt in 0..FIRST_TRY + MAX_RETRIES {
            let mut response =
                self.execute_with_csrf_retry(|client| Ok(client.get(url.as_str()).build()?))?;
            let body = response.text()?;
            let operation_status_response: RawOperationStatusResponse = serde_json::from_str(&body)
                .map_err(|source| RobloxApiError::BadResponseJson {
                    body: body.clone(),
                    source,
                })?;

            match operation_status_response.response {
                Some(variants) => match variants {
                    RawOperationStatusResponseVariants::Success { asset_id, .. } => {
                        return Ok(asset_id);
                    }
                    RawOperationStatusResponseVariants::Failure { code, message } => {
                        return Err(RobloxApiError::ApiError {
                            message: format!("Operation failed: {}: {}", code, message),
                        })
                    }
                },
                None => {
                    let delay = BASE_DELAY + STEP_DELAY * (attempt.pow(EXPONENTIAL_BACKOFF));
                    std::thread::sleep(delay);
                }
            };
        }

        Err(RobloxApiError::ApiError {
            message: format!(
                "polling operation: {} did not complete in time",
                operation_id
            ),
        })
    }
}

#[derive(Debug, Error)]
pub enum RobloxApiError {
    #[error("Roblox API HTTP error")]
    Http {
        #[from]
        source: reqwest::Error,
    },

    #[error("Roblox API error: {message}")]
    ApiError { message: String },

    #[error("Roblox API returned success, but had malformed JSON response: {body}")]
    BadResponseJson {
        body: String,
        source: serde_json::Error,
    },

    #[error("Roblox API returned HTTP {status} with body: {body}")]
    ResponseError { status: StatusCode, body: String },
}
