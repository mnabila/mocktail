//! Request log entity + RequestId VO.

use crate::shared::ids::{CollectionId, Id, MockId, WorkspaceId};
use crate::shared::ports::UnixMillis;

/// A request id echoed to the client via the `X-Request-Id` header and used as
/// the lookup key in the portal.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct RequestId(pub String);

impl RequestId {
    /// Honor an inbound id only if it is well-formed: non-empty, <=200 chars,
    /// printable ASCII without control characters.
    pub fn accept_inbound(raw: &str) -> Option<Self> {
        let raw = raw.trim();
        if raw.is_empty() || raw.len() > 200 {
            return None;
        }
        if raw.chars().all(|c| c.is_ascii_graphic() || c == ' ') {
            Some(RequestId(raw.to_string()))
        } else {
            None
        }
    }

    pub fn as_str(&self) -> &str {
        &self.0
    }
}

impl std::fmt::Display for RequestId {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.write_str(self.as_str())
    }
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct ResponseLog {
    pub status: u16,
    pub headers: Vec<(String, String)>,
    /// UTF-8-lossy body; the testing portal shows it verbatim.
    pub body: String,
}

#[derive(Clone, Debug, PartialEq)]
pub struct RequestLog {
    pub id: Id,
    pub request_id: RequestId,
    pub workspace_id: WorkspaceId,
    pub collection_id: Option<CollectionId>,
    pub received_at: UnixMillis,
    pub method: String,
    pub path: String,
    /// Full inbound headers, in wire order.
    pub request_headers: Vec<(String, String)>,
    /// Full inbound body, UTF-8-lossy.
    pub request_body: String,
    pub matched_mock_id: Option<MockId>,
    /// Why this mock answered (the precedence reason).
    pub match_reason: Option<String>,
    pub status: u16,
    /// Full outbound response (headers + body).
    pub response: ResponseLog,
    pub ip: Option<String>,
    pub country: Option<String>,
    pub lat: Option<f64>,
    pub lng: Option<f64>,
}
