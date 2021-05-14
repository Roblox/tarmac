use std::{borrow::Cow, cmp::max, io, path::Path, thread, time::Duration};

use fs_err as fs;
use reqwest::StatusCode;
use thiserror::Error;

use crate::roblox_web_api::{ImageUploadData, RobloxApiClient, RobloxApiError};

pub trait SyncBackend {
    fn upload(&mut self, data: UploadInfo) -> Result<UploadResponse, Error>;
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct UploadResponse {
    pub id: u64,
}

#[derive(Clone, Debug)]
pub struct UploadInfo {
    pub name: String,
    pub contents: Vec<u8>,
    pub hash: String,
}

pub struct RobloxSyncBackend<'a> {
    api_client: &'a mut RobloxApiClient,
    upload_to_group_id: Option<u64>,
}

impl<'a> RobloxSyncBackend<'a> {
    pub fn new(api_client: &'a mut RobloxApiClient, upload_to_group_id: Option<u64>) -> Self {
        Self {
            api_client,
            upload_to_group_id,
        }
    }
}

impl<'a> SyncBackend for RobloxSyncBackend<'a> {
    fn upload(&mut self, data: UploadInfo) -> Result<UploadResponse, Error> {
        log::info!("Uploading {} to Roblox", &data.name);

        let result = self
            .api_client
            .upload_image_with_moderation_retry(ImageUploadData {
                image_data: Cow::Owned(data.contents),
                name: &data.name,
                description: "Uploaded by Tarmac.",
                group_id: self.upload_to_group_id,
            });

        match result {
            Ok(response) => {
                log::info!(
                    "Uploaded {} to ID {}",
                    &data.name,
                    response.backing_asset_id
                );

                Ok(UploadResponse {
                    id: response.backing_asset_id,
                })
            }

            Err(RobloxApiError::ResponseError {
                status: StatusCode::TOO_MANY_REQUESTS,
                headers,
                ..
            }) => Err(Error::RateLimited {
                wait_seconds: headers
                    .get("retry-after")
                    .ok_or(RobloxApiError::MissingRetryHeader)?
                    .to_str()
                    .unwrap()
                    .parse()
                    .ok(),
            }),

            Err(err) => Err(err.into()),
        }
    }
}

pub struct NoneSyncBackend;

impl SyncBackend for NoneSyncBackend {
    fn upload(&mut self, _data: UploadInfo) -> Result<UploadResponse, Error> {
        Err(Error::NoneBackend)
    }
}

pub struct DebugSyncBackend {
    last_id: u64,
}

impl DebugSyncBackend {
    pub fn new() -> Self {
        Self { last_id: 0 }
    }
}

impl SyncBackend for DebugSyncBackend {
    fn upload(&mut self, data: UploadInfo) -> Result<UploadResponse, Error> {
        log::info!("Copying {} to local folder", &data.name);

        self.last_id += 1;
        let id = self.last_id;

        let path = Path::new(".tarmac-debug");
        fs::create_dir_all(path)?;

        let file_path = path.join(id.to_string());
        fs::write(&file_path, &data.contents)?;

        Ok(UploadResponse { id })
    }
}

/// Performs the retry logic for rate limitation errors. The struct wraps a SyncBackend so that
/// when a RateLimited error occurs, the thread sleeps for a moment and then tries to reupload the
/// data.
pub struct RetryBackend<InnerSyncBackend> {
    inner: InnerSyncBackend,
    min_delay: Duration,
    max_attempts: usize,
}

impl<InnerSyncBackend> RetryBackend<InnerSyncBackend> {
    /// Creates a new backend from another SyncBackend. The max_retries parameter gives the number
    /// of times the backend will try again (so given 0, it acts just as the original SyncBackend).
    /// The delay parameter provides the amount of time to wait between each upload attempt.
    pub fn new(inner: InnerSyncBackend, retries: usize, min_delay: Duration) -> Self {
        Self {
            inner,
            min_delay,
            max_attempts: retries + 1,
        }
    }
}

impl<InnerSyncBackend: SyncBackend> SyncBackend for RetryBackend<InnerSyncBackend> {
    fn upload(&mut self, data: UploadInfo) -> Result<UploadResponse, Error> {
        for index in 1..=self.max_attempts {
            let result = self.inner.upload(data.clone());
            match result {
                Err(Error::RateLimited { wait_seconds }) => {
                    let time = max(self.min_delay, Duration::new(wait_seconds.unwrap_or(0), 0));
                    log::info!(
                        "tarmac is being rate limited, retrying upload after {:?} ({} of {} tries failed)",
                        time,
                        index,
                        self.max_attempts
                    );
                    thread::sleep(time);
                    if index == self.max_attempts {
                        return Err(Error::RateLimited { wait_seconds });
                    }
                }
                _ => return result,
            }
        }
        Err(Error::RateLimited { wait_seconds: None })
    }
}

#[derive(Debug, Error)]
pub enum Error {
    #[error("Cannot upload assets with the 'none' target.")]
    NoneBackend,

    #[error("Tarmac was rate-limited trying to upload assets.{}", .wait_seconds.map_or(String::from(""), |seconds| format!(" Try again in {} seconds.", seconds)))]
    RateLimited { wait_seconds: Option<u64> },

    #[error(transparent)]
    Io {
        #[from]
        source: io::Error,
    },

    #[error(transparent)]
    RobloxError {
        #[from]
        source: RobloxApiError,
    },
}

#[cfg(test)]
mod test {
    use super::*;

    #[allow(unused_must_use)]
    mod test_retry_backend {
        use super::*;

        struct CountUploads<'a> {
            counter: &'a mut usize,
            results: Vec<Result<UploadResponse, Error>>,
        }

        impl<'a> CountUploads<'a> {
            fn new(counter: &'a mut usize) -> Self {
                Self {
                    counter,
                    results: Vec::new(),
                }
            }

            fn with_results(mut self, results: Vec<Result<UploadResponse, Error>>) -> Self {
                self.results = results;
                self.results.reverse();
                self
            }
        }

        impl<'a> SyncBackend for CountUploads<'a> {
            fn upload(&mut self, _data: UploadInfo) -> Result<UploadResponse, Error> {
                (*self.counter) += 1;
                self.results.pop().unwrap_or(Err(Error::NoneBackend))
            }
        }

        fn any_upload_info() -> UploadInfo {
            UploadInfo {
                name: "foo".to_owned(),
                contents: Vec::new(),
                hash: "hash".to_owned(),
            }
        }

        fn retry_duration() -> Duration {
            Duration::from_millis(1)
        }

        #[test]
        fn upload_at_least_once() {
            let mut counter = 0;
            let mut backend =
                RetryBackend::new(CountUploads::new(&mut counter), 0, retry_duration());

            backend.upload(any_upload_info());

            assert_eq!(counter, 1);
        }

        #[test]
        fn upload_again_if_rate_limited() {
            let mut counter = 0;
            let inner = CountUploads::new(&mut counter).with_results(vec![
                Err(Error::RateLimited {
                    wait_seconds: Some(10),
                }),
                Err(Error::RateLimited {
                    wait_seconds: Some(5),
                }),
                Err(Error::NoneBackend),
            ]);
            let mut backend = RetryBackend::new(inner, 5, retry_duration());

            backend.upload(any_upload_info());

            assert_eq!(counter, 3);
        }

        #[test]
        fn upload_returns_first_success_result() {
            let mut counter = 0;
            let success = UploadResponse { id: 10 };
            let inner = CountUploads::new(&mut counter).with_results(vec![
                Err(Error::RateLimited {
                    wait_seconds: Some(10),
                }),
                Err(Error::RateLimited {
                    wait_seconds: Some(5),
                }),
                Ok(success.clone()),
            ]);
            let mut backend = RetryBackend::new(inner, 5, retry_duration());

            let upload_result = backend.upload(any_upload_info()).unwrap();

            assert_eq!(counter, 3);
            assert_eq!(upload_result, success);
        }

        #[test]
        fn upload_returns_rate_limited_when_retries_exhausted() {
            let mut counter = 0;
            let inner = CountUploads::new(&mut counter).with_results(vec![
                Err(Error::RateLimited {
                    wait_seconds: Some(10),
                }),
                Err(Error::RateLimited {
                    wait_seconds: Some(10),
                }),
                Err(Error::RateLimited {
                    wait_seconds: Some(10),
                }),
                Err(Error::RateLimited {
                    wait_seconds: Some(10),
                }),
            ]);
            let mut backend = RetryBackend::new(inner, 2, retry_duration());

            let upload_result = backend.upload(any_upload_info()).unwrap_err();

            assert_eq!(counter, 3);
            assert!(match upload_result {
                Error::RateLimited { .. } => true,
                _ => false,
            });
        }
    }
}
