use serde::{Deserialize, Serialize};
use std::borrow::Cow;
use thiserror::Error;

#[derive(Debug, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct CreationContext {
    pub creator: Creator,
}

#[derive(Debug, Serialize, Deserialize)]
#[serde(rename_all = "camelCase", untagged)]
pub enum Creator {
    #[serde(rename_all = "camelCase")]
    User { user_id: String },
    #[serde(rename_all = "camelCase")]
    Group { group_id: String },
}

#[derive(Debug, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct RawOperationStatusResponse {
    #[serde(flatten)]
    pub common: RawOperationStatusResponseCommon,
    pub response: Option<RawOperationStatusResponseVariants>,
}
#[derive(Debug, Serialize, Deserialize)]
#[serde(untagged)]
pub enum RawOperationStatusResponseVariants {
    #[serde(rename_all = "camelCase")]
    Success {
        path: String,
        revision_id: String,
        revision_create_time: String,
        asset_id: String,
        display_name: String,
        description: String,
        asset_type: String,
        creation_context: CreationContext,
        moderation_result: ModerationResult,
        state: String,
    },
    // InProgress, InProgress is represented by None
    #[serde(rename_all = "camelCase")]
    Failure { code: String, message: String },
}

#[derive(Debug, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct RawOperationStatusResponseCommon {
    path: String,
    operation_id: String,
    done: bool,
}

#[derive(Debug, Deserialize, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct ModerationResult {
    moderation_state: String,
}

pub struct ImageUploadData<'a> {
    pub image_data: Cow<'a, [u8]>,
    pub image_metadata: ImageUploadMetadata,
}

#[derive(Debug, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct ImageUploadMetadata {
    pub asset_type: String,
    pub display_name: String,
    pub description: String,
    pub creation_context: CreationContext,
}

impl ImageUploadMetadata {
    pub fn new(
        asset_type: String,
        display_name: String,
        description: String,
        user_id: Option<u64>,
        group_id: Option<u64>,
    ) -> Result<Self, RobloxAuthenticationError> {
        let creator = match (user_id, group_id) {
            (Some(user_id), None) => Creator::User {
                user_id: user_id.to_string(),
            },
            (None, Some(group_id)) => Creator::Group {
                group_id: group_id.to_string(),
            },
            _ => return Err(RobloxAuthenticationError::InvalidCreatorIdProvided),
        };
        Ok(Self {
            asset_type: asset_type.to_string(),
            display_name: display_name.to_string(),
            description: description.to_string(),
            creation_context: CreationContext { creator },
        })
    }
}

#[derive(Debug, Serialize, Deserialize)]
#[serde(rename_all = "PascalCase")]
pub struct UploadResponse {
    pub asset_id: u64,
}

/// Internal representation of what the asset upload endpoint returns, before
/// we've handled any errors.
#[derive(Debug, Deserialize)]
#[serde(rename_all = "camelCase", untagged)]
pub enum RawUploadResponse {
    #[serde(rename_all = "camelCase")]
    Success {
        path: String,
        operation_id: String,
        done: bool,
    },
    #[serde(rename_all = "camelCase")]
    Error { code: String, message: String },
}

#[derive(Debug, Error)]
pub enum RobloxAuthenticationError {
    #[error("Exactly one of user_id or group_id must be provided")]
    InvalidCreatorIdProvided,
    #[error("Exactly one of api_key or auth must be provided")]
    InvalidAuthProvided,
}
