use common_utils::pii;
use masking::Secret;

use crate::user_role::UserStatus;
pub mod dashboard_metadata;
#[cfg(feature = "dummy_connector")]
pub mod sample_data;

#[derive(serde::Deserialize, Debug, Clone, serde::Serialize)]
pub struct ConnectAccountRequest {
    pub email: pii::Email,
    pub password: Secret<String>,
}

#[derive(serde::Serialize, Debug, Clone)]
pub struct ConnectAccountResponse {
    pub token: Secret<String>,
    pub merchant_id: String,
    pub name: Secret<String>,
    pub email: pii::Email,
    pub verification_days_left: Option<i64>,
    pub user_role: String,
    //this field is added for audit/debug reasons
    #[serde(skip_serializing)]
    pub user_id: String,
}

#[derive(serde::Deserialize, Debug, serde::Serialize)]
pub struct ChangePasswordRequest {
    pub new_password: Secret<String>,
    pub old_password: Secret<String>,
}

#[derive(Debug, serde::Deserialize, serde::Serialize)]
pub struct SwitchMerchantIdRequest {
    pub merchant_id: String,
}

#[derive(serde::Deserialize, Debug, serde::Serialize)]
pub struct CreateInternalUserRequest {
    pub name: Secret<String>,
    pub email: pii::Email,
    pub password: Secret<String>,
}

#[derive(Debug, serde::Deserialize, serde::Serialize)]
pub struct UserMerchantCreate {
    pub company_name: String,
}

#[derive(Debug, serde::Serialize)]
pub struct GetUsersResponse(pub Vec<UserDetails>);

#[derive(Debug, serde::Serialize)]
pub struct UserDetails {
    pub user_id: String,
    pub email: pii::Email,
    pub name: Secret<String>,
    pub role_id: String,
    pub role_name: String,
    pub status: UserStatus,
    #[serde(with = "common_utils::custom_serde::iso8601")]
    pub last_modified_at: time::PrimitiveDateTime,
}
