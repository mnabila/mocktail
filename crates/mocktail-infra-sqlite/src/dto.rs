//! Serde document types persisted as JSON in the SQLite `doc` column.
//! Ids are plain strings and dates are integers, so no EJSON coercion is needed.
#![allow(non_snake_case)] // document fields are camelCase

use serde::{Deserialize, Serialize};

#[derive(Serialize, Deserialize)]
pub struct WorkspaceDoc {
    pub id: String,
    pub name: String,
    pub slug: String,
    pub created_at: i64,
}

#[derive(Serialize, Deserialize)]
pub struct CorsDoc {
    pub allowOriginsAny: bool,
    pub allowOriginsList: Vec<String>,
    pub allowMethods: Vec<String>,
    pub allowHeaders: Vec<String>,
    pub exposeHeaders: Vec<String>,
    pub allowCredentials: bool,
    pub maxAgeSecs: u32,
}

#[derive(Serialize, Deserialize)]
pub struct CollectionDoc {
    pub id: String,
    pub workspace_id: String,
    pub name: String,
    pub slug: String,
    pub description: String,
    pub source: String,
    pub base_path: String,
    pub cors: CorsDoc,
    pub version: i64,
    pub created_at: i64,
}

#[derive(Serialize, Deserialize)]
pub struct MatcherDoc {
    pub targetType: String,
    pub targetName: Option<String>,
    pub kindType: String,
    pub value: Option<String>,
    pub path: Option<String>,
    pub equals: Option<String>,
}

#[derive(Serialize, Deserialize)]
pub struct ResponseDoc {
    pub status: u16,
    pub headers: Vec<(String, String)>,
    pub body: String,
}

#[derive(Serialize, Deserialize)]
pub struct RateLimitDoc {
    #[serde(default)]
    pub limit: u32,
    #[serde(default = "default_rate_window")]
    pub windowSecs: u32,
    #[serde(default = "default_rate_status")]
    pub status: u16,
}

fn default_rate_window() -> u32 {
    60
}

fn default_rate_status() -> u16 {
    429
}

#[derive(Serialize, Deserialize)]
pub struct MockDoc {
    pub id: String,
    pub workspace_id: String,
    pub collection_id: String,
    #[serde(default)]
    pub folder_id: Option<String>,
    pub method: String,
    pub path_template: String,
    pub matchers: Vec<MatcherDoc>,
    pub response: ResponseDoc,
    pub priority: i32,
    pub delay_ms: u32,
    pub fault: Option<String>,
    #[serde(default)]
    pub rate_limit: Option<RateLimitDoc>,
    pub created_at: i64,
    pub updated_at: i64,
}

#[derive(Serialize, Deserialize)]
pub struct FolderDoc {
    pub id: String,
    pub workspace_id: String,
    pub collection_id: String,
    pub name: String,
    #[serde(default)]
    pub parent_id: Option<String>,
    pub created_at: i64,
}

#[derive(Serialize, Deserialize)]
pub struct IdentityDoc {
    pub provider: String,
    pub subject: String,
    pub email: Option<String>,
}

#[derive(Serialize, Deserialize)]
pub struct UserDoc {
    pub id: String,
    pub username: String,
    pub display_name: String,
    pub created_at: i64,
}

#[derive(Serialize, Deserialize)]
pub struct MembershipDoc {
    pub id: String,
    pub user_id: String,
    pub workspace_id: String,
    pub role: String,
}

#[derive(Serialize, Deserialize, Default)]
pub struct ResponseLogDoc {
    #[serde(default)]
    pub status: u16,
    #[serde(default)]
    pub headers: Vec<(String, String)>,
    #[serde(default)]
    pub body: String,
}

#[derive(Serialize, Deserialize)]
pub struct RequestLogDoc {
    pub id: String,
    pub request_id: String,
    pub workspace_id: String,
    pub collection_id: Option<String>,
    pub received_at: i64,
    pub method: String,
    pub path: String,
    #[serde(default)]
    pub request_headers: Vec<(String, String)>,
    #[serde(default)]
    pub request_body: String,
    pub matched_mock_id: Option<String>,
    pub match_reason: Option<String>,
    pub status: u16,
    #[serde(default)]
    pub response: ResponseLogDoc,
    #[serde(default)]
    pub ip: Option<String>,
    #[serde(default)]
    pub country: Option<String>,
    #[serde(default)]
    pub lat: Option<f64>,
    #[serde(default)]
    pub lng: Option<f64>,
}
