//! Low Level API Wrapper
use crate::error::PantryError;
use futures::stream::{Stream, StreamExt, TryStreamExt};
use hyper;
use hyper::body::HttpBody;
use hyper::Client;
use hyper::StatusCode;
use hyperlocal::UnixClientExt;

use serde_json;
use serde_json::Value;
use sse_codec::{decode_stream, Event};
use std::collections::HashMap;
use std::fmt;
use std::io; // for try_next()
use std::pin::Pin;
use uuid::Uuid;

use crate::interface::{
    LLMEvent, LLMRegistryEntry, LLMRunningStatus, LLMStatus, UserInfo, UserPermissions,
    UserRequestStatus,
};

#[derive(Debug, serde::Serialize, serde::Deserialize)]
struct RegisterUserRequest {
    user_name: String,
}

/// Enum representing valid capability ratings for LLMs.
#[derive(serde::Serialize, serde::Deserialize, Clone, Copy, Debug)]
#[serde(rename_all = "lowercase")]
pub enum CapabilityType {
    General,
    Assistant,
    Writing,
    Coding,
}

impl fmt::Display for CapabilityType {
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        match self {
            CapabilityType::General => write!(f, "general"),
            CapabilityType::Assistant => write!(f, "assistant"),
            CapabilityType::Writing => write!(f, "writing"),
            CapabilityType::Coding => write!(f, "coding"),
        }
    }
}

/// Filter structure for capabilities, for use when
/// describing LLM filters or preferences.
#[derive(Debug, serde::Deserialize, serde::Serialize)]
pub struct CapabilityFilter {
    pub capability: CapabilityType,
    pub value: i32,
}

/// Filter for calls that allow flexible choice of LLMs.
///
/// Filter qualification are match-or-fail, meaning if a
/// filter cannot be satisfied, the function will return a 404.
///
/// An empty filter structure will allow any LLM to be used.
#[derive(Debug, serde::Deserialize, serde::Serialize)]
pub struct LLMFilter {
    /// UUID. This specifies a single LLM, making the rest of the options unnecessary.
    pub llm_uuid: Option<Uuid>,
    pub llm_id: Option<String>,
    pub family_id: Option<String>,
    pub local: Option<bool>,
    pub minimum_capabilities: Option<Vec<CapabilityFilter>>,
}

/// Preference structure for calls that allow flexible choice of LLMs.
///
/// Preferences are sort-then-choose, meaning you're preferences
/// will _not_ entirely exclude any result.
///
/// Preferences are evaluated in the following order:
/// * uuid
/// * llm_id
/// * local
/// * family_id
/// * capability_type
///
/// This means that if a uuid matches, it gets returned, otherwise
/// the other preferences get evaluated. If more than one LLM matches,
/// the results are filtered to those LLMs and the next preference
/// is applied. If no capability type is provided, the final sorting
/// (should multiple LLMs be left over) is based on [CapabilityType::General].
#[derive(Debug, serde::Deserialize, serde::Serialize)]
pub struct LLMPreference {
    pub llm_uuid: Option<Uuid>,
    pub llm_id: Option<String>,
    pub local: Option<bool>,
    pub family_id: Option<String>,
    pub capability_type: Option<CapabilityType>,
}

#[derive(Debug, serde::Serialize, serde::Deserialize)]
struct RequestPermissionRequest {
    user_id: String,
    api_key: String,
    requested_permissions: UserPermissions, // You might want to replace this with an actual Permissions type
}

#[derive(Debug, serde::Serialize, serde::Deserialize)]
struct RequestDownloadRequest {
    user_id: String,
    api_key: String,
    llm_registry_entry: String, // You might want to replace this with an actual LLMRegistryEntry type
}

#[derive(Debug, serde::Serialize, serde::Deserialize)]
struct RequestLoadRequest {
    user_id: String,
    api_key: String,
    llm_id: String,
}

#[derive(Debug, serde::Serialize, serde::Deserialize)]
struct RequestLoadFlexRequest {
    user_id: String,
    api_key: String,
    filter: Option<LLMFilter>,         // Replace with actual LLMFilter type
    preference: Option<LLMPreference>, // Replace with actual LLMPreference type
}

#[derive(Debug, serde::Serialize, serde::Deserialize)]
struct RequestUnloadRequest {
    user_id: String,
    api_key: String,
    llm_id: String,
}

#[derive(Debug, serde::Serialize, serde::Deserialize)]
struct LoadLLMRequest {
    user_id: String,
    api_key: String,
    llm_id: String,
}

#[derive(Debug, serde::Serialize, serde::Deserialize)]
struct UnloadLLMRequest {
    user_id: String,
    api_key: String,
    llm_id: String,
}

#[derive(Debug, serde::Serialize, serde::Deserialize)]
struct DownloadLLMRequest {
    user_id: String,
    api_key: String,
    llm_registry_entry: String, // You might want to replace this with an actual LLMRegistryEntry type
}

#[derive(Debug, serde::Deserialize, serde::Serialize)]
struct RequestStatusRequest {
    user_id: String,
    api_key: String,
    request_id: String,
}
#[derive(Debug, serde::Deserialize, serde::Serialize)]
struct LoadLLMFlexRequest {
    user_id: String,
    api_key: String,
    filter: Option<LLMFilter>,
    preference: Option<LLMPreference>,
}

#[derive(Debug, serde::Serialize, serde::Deserialize)]
struct CreateSessionRequest {
    user_id: String,
    api_key: String,
    user_session_parameters: HashMap<String, Value>,
}

#[derive(Debug, serde::Serialize, serde::Deserialize)]
struct CreateSessionIdRequest {
    user_id: String,
    api_key: String,
    llm_id: String,
    user_session_parameters: HashMap<String, Value>,
}

#[derive(Debug, serde::Serialize, serde::Deserialize)]
struct CreateSessionFlexRequest {
    user_id: String,
    api_key: String,
    filter: Option<LLMFilter>,         // Replace with actual LLMFilter type
    preference: Option<LLMPreference>, // Replace with actual LLMPreference type
    user_session_parameters: HashMap<String, Value>,
}

#[derive(Debug, serde::Deserialize, serde::Serialize)]
pub struct CreateSessionResponse {
    pub session_parameters: HashMap<String, Value>,
    pub llm_status: LLMStatus,
    pub session_id: String,
}

#[derive(Debug, serde::Serialize, serde::Deserialize)]
struct PromptSessionStreamRequest {
    user_id: String,
    api_key: String,
    session_id: String,
    llm_uuid: String,
    prompt: String,
    parameters: HashMap<String, Value>,
}

#[derive(Debug, serde::Serialize, serde::Deserialize)]
struct GetLLMStatusRequest {
    user_id: String,
    api_key: String,
    llm_id: String,
}

#[derive(Debug, serde::Serialize, serde::Deserialize)]
struct GetAvailableLLMRequest {
    user_id: String,
    api_key: String,
}

#[derive(Debug, serde::Serialize, serde::Deserialize)]
struct InterruptSessionRequest {
    user_id: String,
    api_key: String,
    llm_uuid: String,
    session_id: String,
}

#[derive(Debug, serde::Serialize, serde::Deserialize)]
struct GetRunningLLMRequest {
    user_id: String,
    api_key: String,
}

#[derive(Debug, serde::Deserialize, serde::Serialize)]
struct BareModelRequest {
    user_id: String,
    api_key: String,
    llm_id: String,
}

#[derive(Debug, serde::Deserialize, serde::Serialize)]
struct BareModelFlexRequest {
    user_id: String,
    api_key: String,
    filter: Option<LLMFilter>,
    preference: Option<LLMPreference>,
}

#[derive(Debug, serde::Deserialize, serde::Serialize)]
pub struct BareModelResponse {
    pub model: LLMStatus,
    pub path: String,
}

/// PantryAPI is a thin wrapper, just meant to minimize retyping of
/// client and baseurl in function calls. Feel free to make multiple,
/// or to clone.
#[derive(Clone)]
pub struct PantryAPI {
    pub client: Client<hyper::client::connect::HttpConnector>,
    pub base_url: String,
}

impl PantryAPI {
    pub fn new(base_url: String) -> Self {
        PantryAPI {
            client: Client::new(),
            base_url,
        }
    }

    async fn double_edge(
        &self,
        method: hyper::Method,
        body: String,
        path: String,
    ) -> Result<hyper::Response<hyper::body::Body>, PantryError> {
        let url1 = hyperlocal::Uri::new("/tmp/pantrylocal.sock", &path.clone());
        let req1: hyper::Request<hyper::body::Body> = hyper::Request::builder()
            .method(method.clone())
            .header("Content-Type", "application/json")
            .uri(url1)
            .body(hyper::Body::from(body.clone()))?;
        let url2 = self.base_url.clone() + &path;
        let req2: hyper::Request<hyper::body::Body> = hyper::Request::builder()
            .method(method.clone())
            .header("Content-Type", "application/json")
            .uri(url2)
            .body(hyper::Body::from(body.clone()))?;

        let unix = Client::unix();

        match unix.request(req1).await {
            Ok(resp) => Ok(resp),
            Err(err) => {
                println!("Error sending to socket: {:?}", err);
                println!("Trying: {:?}", req2);
                Ok(self.client.request(req2).await?)
            }
        }
    }

    /// Accessing the API requires a registered user demarcated by a user_id and an api_key.
    ///
    /// This function supplies both. When using the API manually, you'll probably also
    /// need to call [PantryAPI::request_permissions] to do anything useful.
    ///
    /// # Arguments
    /// * `user_name` — used for debug output and manager display.
    pub async fn register_user(&self, user_name: String) -> Result<UserInfo, PantryError> {
        let register_user_request = RegisterUserRequest { user_name };

        let body = serde_json::to_string(&register_user_request)?;
        let resp = self
            .double_edge(hyper::Method::POST, body, format!("/register_user"))
            .await?;
        match resp.status() {
            StatusCode::OK => {
                // Get the response body bytes.
                let body_bytes = hyper::body::to_bytes(resp.into_body()).await?;

                // Convert the body bytes to utf-8
                // let body = String::from_slice(body_bytes.into()).unwrap();
                let body_str = std::str::from_utf8(&body_bytes)?;
                Ok(serde_json::from_str(body_str)?)
            }
            code => {
                let body_bytes = hyper::body::to_bytes(resp.into_body()).await?;

                // Convert the body bytes to utf-8
                // let body = String::from_slice(body_bytes.into()).unwrap();
                let body_str = std::str::from_utf8(&body_bytes)?;

                Err(PantryError::ApiError(code, body_str.into()))
            }
        }

        // let damn: UserInfo = serde_json::from_slice(ff).unwrap();
        // Ok(serde_json::from_slice(&ff)?)
    }

    /// Requests permissions. See the [UserPermissions] struct for more details.
    /// The system owner must accept the request (currently in the UI).
    ///
    /// # Arguments
    ///
    /// * `user_id` — A UUID, obtained from [PantryAPI::register_user].
    /// * `api_key` — An API key, obtained from [PantryAPI::register_user]
    /// * `requested_permissions` — The permissions this api user wants.
    pub async fn request_permissions(
        &self,
        user_id: Uuid,
        api_key: String,
        requested_permissions: UserPermissions,
    ) -> Result<UserRequestStatus, PantryError> {
        let request_permission_request = RequestPermissionRequest {
            user_id: user_id.to_string(),
            api_key,
            requested_permissions,
        };
        let body = serde_json::to_string(&request_permission_request)?;
        let resp = self
            .double_edge(hyper::Method::POST, body, format!("/request_permissions"))
            .await?;
        match resp.status() {
            StatusCode::OK => {
                // Get the response body bytes.
                let body_bytes = hyper::body::to_bytes(resp.into_body()).await?;

                // Convert the body bytes to utf-8
                // let body = String::from_slice(body_bytes.into()).unwrap();
                let body_str = std::str::from_utf8(&body_bytes)?;
                Ok(serde_json::from_str(&body_str)?)
            }
            code => {
                let body_bytes = hyper::body::to_bytes(resp.into_body()).await?;

                // Convert the body bytes to utf-8
                // let body = String::from_slice(body_bytes.into()).unwrap();
                let body_str = std::str::from_utf8(&body_bytes)?;

                Err(PantryError::ApiError(code, body_str.into()))
            }
        }
    }

    /// Creates a request to download a new model. Must be accepted by the system
    /// owner (currently via the UI).
    ///
    /// # Arguments
    ///
    /// * `user_id` — A UUID, obtained from [PantryAPI::register_user].
    /// * `api_key` — An API key, obtained from [PantryAPI::register_user]
    /// * `llm_registry_entry` — A valid LLM registry entry to download. This specifies
    /// the location of the model as well as any metadata. For better usability, try
    /// being comprehensive about this.
    pub async fn request_download(
        &self,
        user_id: Uuid,
        api_key: String,
        llm_registry_entry: LLMRegistryEntry,
    ) -> Result<UserRequestStatus, PantryError> {
        let request_download_request = RequestDownloadRequest {
            user_id: user_id.to_string(),
            api_key,
            llm_registry_entry: serde_json::to_string(&llm_registry_entry)?,
        };
        let body = serde_json::to_string(&request_download_request)?;
        let resp = self
            .double_edge(hyper::Method::POST, body, format!("/request_download"))
            .await?;
        match resp.status() {
            StatusCode::OK => {
                // Get the response body bytes.
                let body_bytes = hyper::body::to_bytes(resp.into_body()).await?;

                // Convert the body bytes to utf-8
                // let body = String::from_slice(body_bytes.into()).unwrap();
                let body_str = std::str::from_utf8(&body_bytes)?;
                Ok(serde_json::from_str(&body_str)?)
            }
            code => {
                let body_bytes = hyper::body::to_bytes(resp.into_body()).await?;

                // Convert the body bytes to utf-8
                // let body = String::from_slice(body_bytes.into()).unwrap();
                let body_str = std::str::from_utf8(&body_bytes)?;

                Err(PantryError::ApiError(code, body_str.into()))
            }
        }
    }

    /// Requests a load, but doesn't predetermine the exact LLM ahead of time.
    ///
    /// # Arguments
    ///
    /// * `user_id` — A UUID, obtained from [PantryAPI::register_user].
    /// * `api_key` — An API key, obtained from [PantryAPI::register_user]
    /// * `filter` — An [LLMFilter] specifying hard requirements for the LLM.
    /// * `preference` — An [LLMPreference] specifying soft requirements for the LLM.
    pub async fn request_load_flex(
        &self,
        user_id: Uuid,
        api_key: String,
        filter: Option<LLMFilter>,
        preference: Option<LLMPreference>,
    ) -> Result<UserRequestStatus, PantryError> {
        let request_load_request = RequestLoadFlexRequest {
            user_id: user_id.to_string(),
            api_key,
            filter,
            preference,
        };
        let body = serde_json::to_string(&request_load_request)?;
        let resp = self
            .double_edge(hyper::Method::POST, body, format!("/request_load"))
            .await?;
        match resp.status() {
            StatusCode::OK => {
                // Get the response body bytes.
                let body_bytes = hyper::body::to_bytes(resp.into_body()).await?;

                // Convert the body bytes to utf-8
                // let body = String::from_slice(body_bytes.into()).unwrap();
                let body_str = std::str::from_utf8(&body_bytes)?;
                Ok(serde_json::from_str(&body_str)?)
            }
            code => {
                let body_bytes = hyper::body::to_bytes(resp.into_body()).await?;

                // Convert the body bytes to utf-8
                // let body = String::from_slice(body_bytes.into()).unwrap();
                let body_str = std::str::from_utf8(&body_bytes)?;

                Err(PantryError::ApiError(code, body_str.into()))
            }
        }
    }

    /// Requests Pantry to load a specific LLM.
    ///
    /// # Arguments
    ///
    /// * `user_id` — A UUID, obtained from [PantryAPI::register_user].
    /// * `api_key` — An API key, obtained from [PantryAPI::register_user]
    /// * `llm_id` — A UUID for the LLM you want to load. Find one via [PantryAPI::get_available_llms].
    pub async fn request_load(
        &self,
        user_id: Uuid,
        api_key: String,
        llm_id: Uuid,
    ) -> Result<UserRequestStatus, PantryError> {
        let request_load_request = RequestLoadRequest {
            user_id: user_id.to_string(),
            api_key,
            llm_id: llm_id.to_string(),
        };
        let body = serde_json::to_string(&request_load_request)?;
        let resp = self
            .double_edge(hyper::Method::POST, body, format!("/request_load"))
            .await?;
        match resp.status() {
            StatusCode::OK => {
                // Get the response body bytes.
                let body_bytes = hyper::body::to_bytes(resp.into_body()).await?;

                // Convert the body bytes to utf-8
                // let body = String::from_slice(body_bytes.into()).unwrap();
                let body_str = std::str::from_utf8(&body_bytes)?;
                Ok(serde_json::from_str(&body_str)?)
            }
            code => {
                let body_bytes = hyper::body::to_bytes(resp.into_body()).await?;

                // Convert the body bytes to utf-8
                // let body = String::from_slice(body_bytes.into()).unwrap();
                let body_str = std::str::from_utf8(&body_bytes)?;

                Err(PantryError::ApiError(code, body_str.into()))
            }
        }
    }

    /// Requests an LLM be shutdown, conserving resources. This should
    /// generally be non-destructive—Pantry attempts to save sessions to disc on shutdown.
    ///
    /// # Arguments
    ///
    /// * `user_id` — A UUID, obtained from [PantryAPI::register_user].
    /// * `api_key` — An API key, obtained from [PantryAPI::register_user]
    /// * `llm_id` — UUID of the LLM. Find running llms via [PantryAPI::get_running_llms].
    pub async fn request_unload(
        &self,
        user_id: Uuid,
        api_key: String,
        llm_id: Uuid,
    ) -> Result<UserRequestStatus, PantryError> {
        let request_unload_request = RequestUnloadRequest {
            user_id: user_id.to_string(),
            api_key,
            llm_id: llm_id.to_string(),
        };
        let body = serde_json::to_string(&request_unload_request)?;
        let resp = self
            .double_edge(hyper::Method::POST, body, format!("/request_unload"))
            .await?;
        match resp.status() {
            StatusCode::OK => {
                // Get the response body bytes.
                let body_bytes = hyper::body::to_bytes(resp.into_body()).await?;

                // Convert the body bytes to utf-8
                // let body = String::from_slice(body_bytes.into()).unwrap();
                let body_str = std::str::from_utf8(&body_bytes)?;
                Ok(serde_json::from_str(&body_str)?)
            }
            code => {
                let body_bytes = hyper::body::to_bytes(resp.into_body()).await?;

                // Convert the body bytes to utf-8
                // let body = String::from_slice(body_bytes.into()).unwrap();
                let body_str = std::str::from_utf8(&body_bytes)?;

                Err(PantryError::ApiError(code, body_str.into()))
            }
        }
    }

    pub async fn get_request_status(
        &self,
        user_id: Uuid,
        api_key: String,
        request_id: Uuid,
    ) -> Result<UserRequestStatus, PantryError> {
        let request_unload_request = RequestStatusRequest {
            user_id: user_id.to_string(),
            api_key,
            request_id: request_id.to_string(),
        };
        let body = serde_json::to_string(&request_unload_request)?;
        let resp = self
            .double_edge(hyper::Method::POST, body, format!("/get_request_status"))
            .await?;
        match resp.status() {
            StatusCode::OK => {
                // Get the response body bytes.
                let body_bytes = hyper::body::to_bytes(resp.into_body()).await?;

                // Convert the body bytes to utf-8
                // let body = String::from_slice(body_bytes.into()).unwrap();
                let body_str = std::str::from_utf8(&body_bytes)?;
                Ok(serde_json::from_str(&body_str)?)
            }
            code => {
                let body_bytes = hyper::body::to_bytes(resp.into_body()).await?;

                // Convert the body bytes to utf-8
                // let body = String::from_slice(body_bytes.into()).unwrap();
                let body_str = std::str::from_utf8(&body_bytes)?;

                Err(PantryError::ApiError(code, body_str.into()))
            }
        }
    }

    /// Gets the current status of an LLM
    ///
    /// # Arguments
    ///
    /// * `user_id` — A UUID, obtained from [PantryAPI::register_user].
    /// * `api_key` — An API key, obtained from [PantryAPI::register_user]
    /// *
    pub async fn get_llm_status(
        &self,
        user_id: Uuid,
        api_key: String,
        llm_id: Uuid,
    ) -> Result<LLMStatus, PantryError> {
        let request_unload_request = GetLLMStatusRequest {
            user_id: user_id.to_string(),
            api_key,
            llm_id: llm_id.to_string(),
        };
        let body = serde_json::to_string(&request_unload_request)?;
        let resp = self
            .double_edge(hyper::Method::POST, body, format!("/get_llm_status"))
            .await?;
        match resp.status() {
            StatusCode::OK => {
                // Get the response body bytes.
                let body_bytes = hyper::body::to_bytes(resp.into_body()).await?;

                // Convert the body bytes to utf-8
                // let body = String::from_slice(body_bytes.into()).unwrap();
                let body_str = std::str::from_utf8(&body_bytes)?;
                Ok(serde_json::from_str(&body_str)?)
            }
            code => {
                let body_bytes = hyper::body::to_bytes(resp.into_body()).await?;

                // Convert the body bytes to utf-8
                // let body = String::from_slice(body_bytes.into()).unwrap();
                let body_str = std::str::from_utf8(&body_bytes)?;

                Err(PantryError::ApiError(code, body_str.into()))
            }
        }
    }

    /// Gets currently running LLMs.
    ///
    /// # Arguments
    ///
    /// * `user_id` — A UUID, obtained from [PantryAPI::register_user].
    /// * `api_key` — An API key, obtained from [PantryAPI::register_user]
    pub async fn get_running_llms(
        &self,
        user_id: Uuid,
        api_key: String,
    ) -> Result<Vec<LLMStatus>, PantryError> {
        let request_running_llms = GetRunningLLMRequest {
            user_id: user_id.to_string(),
            api_key,
        };
        let body = serde_json::to_string(&request_running_llms)?;
        let resp = self
            .double_edge(hyper::Method::POST, body, format!("/get_running_llms"))
            .await?;
        match resp.status() {
            StatusCode::OK => {
                // Get the response body bytes.
                let body_bytes = hyper::body::to_bytes(resp.into_body()).await?;

                // Convert the body bytes to utf-8
                // let body = String::from_slice(body_bytes.into()).unwrap();
                let body_str = std::str::from_utf8(&body_bytes)?;
                Ok(serde_json::from_str(&body_str)?)
            }
            code => {
                let body_bytes = hyper::body::to_bytes(resp.into_body()).await?;

                // Convert the body bytes to utf-8
                // let body = String::from_slice(body_bytes.into()).unwrap();
                let body_str = std::str::from_utf8(&body_bytes)?;

                Err(PantryError::ApiError(code, body_str.into()))
            }
        }
    }

    /// Gets currently downloaded LLMs.
    ///
    /// In order to create a session, these must first be activated, requiring the
    /// correct permissions. If you're the system owner, you can also activate
    /// them manually using the UI.
    ///
    /// # Arguments
    ///
    /// * `user_id` — A UUID, obtained from [PantryAPI::register_user].
    /// * `api_key` — An API key, obtained from [PantryAPI::register_user]
    pub async fn get_available_llms(
        &self,
        user_id: Uuid,
        api_key: String,
    ) -> Result<Vec<LLMStatus>, PantryError> {
        let request_available_llms = GetAvailableLLMRequest {
            user_id: user_id.to_string(),
            api_key,
        };
        let body = serde_json::to_string(&request_available_llms)?;
        let resp = self
            .double_edge(hyper::Method::POST, body, format!("/get_available_llms"))
            .await?;
        match resp.status() {
            StatusCode::OK => {
                // Get the response body bytes.
                let body_bytes = hyper::body::to_bytes(resp.into_body()).await?;

                // Convert the body bytes to utf-8
                // let body = String::from_slice(body_bytes.into()).unwrap();
                let body_str = std::str::from_utf8(&body_bytes)?;
                Ok(serde_json::from_str(&body_str)?)
            }
            code => {
                let body_bytes = hyper::body::to_bytes(resp.into_body()).await?;

                // Convert the body bytes to utf-8
                // let body = String::from_slice(body_bytes.into()).unwrap();
                let body_str = std::str::from_utf8(&body_bytes)?;

                Err(PantryError::ApiError(code, body_str.into()))
            }
        }
    }

    /// Interrupts an ongoing inference session.
    ///
    /// Internally this uses a cancellation callback to cancel inference _after the next token_.
    /// Depending on your system, this might take a moment, especially given that some
    /// tokens might already be inferred but not yet transmitted.
    ///
    /// You must own the session to interrupt it.
    ///
    /// # Arguments
    ///
    /// * `user_id` — A UUID, obtained from [PantryAPI::register_user].
    /// * `api_key` — An API key, obtained from [PantryAPI::register_user]
    /// * `llm_id` — A UUID of an LLM. You should have gotten it from creating your session.
    /// * `session_id` — A UUID of a session. You should have gotten it from creating your session.
    pub async fn interrupt_session(
        &self,
        user_id: Uuid,
        api_key: String,
        llm_id: Uuid,
        session_id: Uuid,
    ) -> Result<LLMRunningStatus, PantryError> {
        let interrupt_session_request = InterruptSessionRequest {
            user_id: user_id.to_string(),
            api_key,
            llm_uuid: llm_id.to_string(),
            session_id: session_id.to_string(),
        };
        let body = serde_json::to_string(&interrupt_session_request)?;
        let resp = self
            .double_edge(hyper::Method::POST, body, format!("/interrupt_session"))
            .await?;
        match resp.status() {
            StatusCode::OK => {
                // Get the response body bytes.
                let body_bytes = hyper::body::to_bytes(resp.into_body()).await?;

                // Convert the body bytes to utf-8
                // let body = String::from_slice(body_bytes.into()).unwrap();
                let body_str = std::str::from_utf8(&body_bytes)?;
                Ok(serde_json::from_str(&body_str)?)
            }
            code => {
                let body_bytes = hyper::body::to_bytes(resp.into_body()).await?;

                // Convert the body bytes to utf-8
                // let body = String::from_slice(body_bytes.into()).unwrap();
                let body_str = std::str::from_utf8(&body_bytes)?;

                Err(PantryError::ApiError(code, body_str.into()))
            }
        }
    }

    /// Loads an LLM.
    ///
    /// Requires the [UserPermissions::perm_load_llm] permission.
    ///
    /// # Arguments
    ///
    /// * `user_id` — A UUID, obtained from [PantryAPI::register_user].
    /// * `api_key` — An API key, obtained from [PantryAPI::register_user]
    /// * `filter` — A [LLMFilter] object, for what _must_ be true of an LLM to load it.
    /// * `preference` — A [LLMPreference] object, for how to rank and then select from the LLMs
    /// that pass the filter.
    pub async fn load_llm_flex(
        &self,
        user_id: Uuid,
        api_key: String,
        filter: Option<LLMFilter>,
        preference: Option<LLMPreference>,
    ) -> Result<LLMRunningStatus, PantryError> {
        let load_llm_request = LoadLLMFlexRequest {
            user_id: user_id.to_string(),
            api_key,
            filter,
            preference,
        };
        let body = serde_json::to_string(&load_llm_request)?;
        let resp = self
            .double_edge(hyper::Method::POST, body, format!("/load_llm_flex"))
            .await?;
        match resp.status() {
            StatusCode::OK => {
                // Get the response body bytes.
                let body_bytes = hyper::body::to_bytes(resp.into_body()).await?;

                // Convert the body bytes to utf-8
                // let body = String::from_slice(body_bytes.into()).unwrap();
                let body_str = std::str::from_utf8(&body_bytes)?;
                Ok(serde_json::from_str(&body_str)?)
            }
            code => {
                let body_bytes = hyper::body::to_bytes(resp.into_body()).await?;

                // Convert the body bytes to utf-8
                // let body = String::from_slice(body_bytes.into()).unwrap();
                let body_str = std::str::from_utf8(&body_bytes)?;

                Err(PantryError::ApiError(code, body_str.into()))
            }
        }
    }

    /// Loads an LLM.
    ///
    /// Requires [UserPermissions::perm_load_llm].
    ///
    /// # Arguments
    ///
    /// * `user_id` — A UUID, obtained from [PantryAPI::register_user].
    /// * `api_key` — An API key, obtained from [PantryAPI::register_user]
    /// * `llm_id` — UUID of an LLM.
    pub async fn load_llm(
        &self,
        user_id: Uuid,
        api_key: String,
        llm_id: Uuid,
    ) -> Result<LLMRunningStatus, PantryError> {
        let load_llm_request = LoadLLMRequest {
            user_id: user_id.to_string(),
            api_key,
            llm_id: llm_id.to_string(),
        };
        let bod = serde_json::to_string(&load_llm_request)?;
        let resp = self
            .double_edge(hyper::Method::POST, bod, format!("/load_llm"))
            .await?;
        match resp.status() {
            StatusCode::OK => {
                // Get the response body bytes.
                let body_bytes = hyper::body::to_bytes(resp.into_body()).await?;

                // Convert the body bytes to utf-8
                // let body = String::from_slice(body_bytes.into()).unwrap();
                let body_str = std::str::from_utf8(&body_bytes)?;
                Ok(serde_json::from_str(&body_str)?)
            }
            code => {
                let body_bytes = hyper::body::to_bytes(resp.into_body()).await?;

                // Convert the body bytes to utf-8
                // let body = String::from_slice(body_bytes.into()).unwrap();
                let body_str = std::str::from_utf8(&body_bytes)?;

                Err(PantryError::ApiError(code, body_str.into()))
            }
        }
    }

    /// Unloads an LLM, conserving resources.
    ///
    /// Requires [UserPermissions::perm_unload_llm].
    ///
    /// # Arguments
    ///
    /// * `user_id` — A UUID, obtained from [PantryAPI::register_user].
    /// * `api_key` — An API key, obtained from [PantryAPI::register_user]
    /// * `llm_id` — UUID or model id of an LLM.
    pub async fn unload_llm(
        &self,
        user_id: Uuid,
        api_key: String,
        llm_id: String,
    ) -> Result<LLMStatus, PantryError> {
        let unload_llm_request = UnloadLLMRequest {
            user_id: user_id.to_string(),
            api_key,
            llm_id,
        };
        let body = serde_json::to_string(&unload_llm_request)?;
        let resp = self
            .double_edge(hyper::Method::POST, body, format!("/unload_llm"))
            .await?;
        match resp.status() {
            StatusCode::OK => {
                // Get the response body bytes.
                let body_bytes = hyper::body::to_bytes(resp.into_body()).await?;

                // Convert the body bytes to utf-8
                // let body = String::from_slice(body_bytes.into()).unwrap();
                let body_str = std::str::from_utf8(&body_bytes)?;
                Ok(serde_json::from_str(&body_str)?)
            }
            code => {
                let body_bytes = hyper::body::to_bytes(resp.into_body()).await?;

                // Convert the body bytes to utf-8
                // let body = String::from_slice(body_bytes.into()).unwrap();
                let body_str = std::str::from_utf8(&body_bytes)?;

                Err(PantryError::ApiError(code, body_str.into()))
            }
        }
    }

    /// Downloads an LLM.
    ///
    /// Requires [UserPermissions::perm_download_llm].
    ///
    /// # Arguments
    ///
    /// * `user_id` — A UUID, obtained from [PantryAPI::register_user].
    /// * `api_key` — An API key, obtained from [PantryAPI::register_user]
    /// * `llm_registry_entry` — [LLMRegistryEntry] for the LLM. The only "mandatory" fields are
    /// the [LLMRegistryEntry::connector_type] and [LLMRegistryEntry::id]. If the connector type is
    /// [crate::interface::LLMConnectorType::LLMrs], config must include the key `model_architecture`. For more
    /// details see the [rustformers/llm
    /// documentation](https://docs.rs/llm/latest/llm/enum.ModelArchitecture.html)
    pub async fn download_llm(
        &self,
        user_id: Uuid,
        api_key: String,
        llm_registry_entry: LLMRegistryEntry,
    ) -> Result<Value, PantryError> {
        let reg_entry_string = serde_json::to_string(&llm_registry_entry)?;
        let download_llm_request = DownloadLLMRequest {
            user_id: user_id.to_string(),
            api_key,
            llm_registry_entry: reg_entry_string,
        };
        let body = serde_json::to_string(&download_llm_request)?;
        let resp = self
            .double_edge(hyper::Method::POST, body, format!("/download_llm"))
            .await?;
        match resp.status() {
            StatusCode::OK => {
                // Get the response body bytes.
                let body_bytes = hyper::body::to_bytes(resp.into_body()).await?;

                // Convert the body bytes to utf-8
                // let body = String::from_slice(body_bytes.into()).unwrap();
                let body_str = std::str::from_utf8(&body_bytes)?;
                Ok(serde_json::from_str(&body_str)?)
            }
            code => {
                let body_bytes = hyper::body::to_bytes(resp.into_body()).await?;

                // Convert the body bytes to utf-8
                // let body = String::from_slice(body_bytes.into()).unwrap();
                let body_str = std::str::from_utf8(&body_bytes)?;

                Err(PantryError::ApiError(code, body_str.into()))
            }
        }
    }

    /// Creates a session, using the best currently running LLM.
    ///
    /// Requires [UserPermissions::perm_session].
    ///
    /// # Arguments
    ///
    /// * `user_id` — A UUID, obtained from [PantryAPI::register_user].
    /// * `api_key` — An API key, obtained from [PantryAPI::register_user]
    /// * `user_session_parameters` — A hashmap of _requested_ parameters. The returning
    /// [LLMStatus] object will inform which ones got accepted by the LLM.
    pub async fn create_session(
        &self,
        user_id: Uuid,
        api_key: String,
        user_session_parameters: HashMap<String, Value>,
    ) -> Result<CreateSessionResponse, PantryError> {
        let create_session_request = CreateSessionRequest {
            user_id: user_id.to_string(),
            api_key,
            user_session_parameters,
        };
        let body = serde_json::to_string(&create_session_request)?;
        let resp = self
            .double_edge(hyper::Method::POST, body, format!("/create_session"))
            .await?;
        match resp.status() {
            StatusCode::OK => {
                // Get the response body bytes.
                let body_bytes = hyper::body::to_bytes(resp.into_body()).await?;

                // Convert the body bytes to utf-8
                // let body = String::from_slice(body_bytes.into()).unwrap();
                let body_str = std::str::from_utf8(&body_bytes)?;
                Ok(serde_json::from_str(&body_str)?)
            }
            code => {
                let body_bytes = hyper::body::to_bytes(resp.into_body()).await?;

                // Convert the body bytes to utf-8
                // let body = String::from_slice(body_bytes.into()).unwrap();
                let body_str = std::str::from_utf8(&body_bytes)?;

                Err(PantryError::ApiError(code, body_str.into()))
            }
        }
    }

    /// Creates a session, using the LLM with the given id. If the LLM doesn't exist or isn't
    /// running, errors.
    ///
    /// Requires [UserPermissions::perm_session].
    ///
    /// # Arguments
    ///
    /// * `user_id` — A UUID, obtained from [PantryAPI::register_user].
    /// * `api_key` — An API key, obtained from [PantryAPI::register_user]
    /// * `llm_id` — A UUID for which LLM to use.
    /// * `user_session_parameters` — A hashmap of _requested_ parameters. The returning
    /// [LLMStatus] object will inform which ones got accepted by the LLM.
    pub async fn create_session_id(
        &self,
        user_id: Uuid,
        api_key: String,
        llm_id: Uuid,
        user_session_parameters: HashMap<String, Value>,
    ) -> Result<CreateSessionResponse, PantryError> {
        let create_session_id_request = CreateSessionIdRequest {
            user_id: user_id.to_string(),
            api_key,
            llm_id: llm_id.to_string(),
            user_session_parameters,
        };
        let body = serde_json::to_string(&create_session_id_request)?;
        let resp = self
            .double_edge(hyper::Method::POST, body, format!("/create_session_id"))
            .await?;
        match resp.status() {
            StatusCode::OK => {
                // Get the response body bytes.
                let body_bytes = hyper::body::to_bytes(resp.into_body()).await?;

                // Convert the body bytes to utf-8
                // let body = String::from_slice(body_bytes.into()).unwrap();
                let body_str = std::str::from_utf8(&body_bytes)?;
                Ok(serde_json::from_str(&body_str)?)
            }
            code => {
                let body_bytes = hyper::body::to_bytes(resp.into_body()).await?;

                // Convert the body bytes to utf-8
                // let body = String::from_slice(body_bytes.into()).unwrap();
                let body_str = std::str::from_utf8(&body_bytes)?;

                Err(PantryError::ApiError(code, body_str.into()))
            }
        }
    }

    /// Creates a session based on `filter` and `preference`. Selects only from currently running
    /// LLMs.
    ///
    /// Requires [UserPermissions::perm_session].
    ///
    /// # Arguments
    ///
    /// * `user_id` — A UUID, obtained from [PantryAPI::register_user].
    /// * `api_key` — An API key, obtained from [PantryAPI::register_user]
    /// * `filter` — A [LLMFilter] object, for what _must_ be true of an LLM to use it.
    /// * `preference` — A [LLMPreference] object, for how to rank and then select from the LLMs
    /// * `user_session_parameters` — A hashmap of _requested_ parameters. The returning
    /// [LLMStatus] object will inform which ones got accepted by the LLM.
    pub async fn create_session_flex(
        &self,
        user_id: Uuid,
        api_key: String,
        filter: Option<LLMFilter>,
        preference: Option<LLMPreference>,
        user_session_parameters: HashMap<String, Value>,
    ) -> Result<CreateSessionResponse, PantryError> {
        let create_session_flex_request = CreateSessionFlexRequest {
            user_id: user_id.to_string(),
            api_key,
            filter,
            preference,
            user_session_parameters,
        };
        let body = serde_json::to_string(&create_session_flex_request)?;
        let resp = self
            .double_edge(hyper::Method::POST, body, format!("/create_session_flex"))
            .await?;
        match resp.status() {
            StatusCode::OK => {
                // Get the response body bytes.
                let body_bytes = hyper::body::to_bytes(resp.into_body()).await?;

                // Convert the body bytes to utf-8
                // let body = String::from_slice(body_bytes.into()).unwrap();
                let body_str = std::str::from_utf8(&body_bytes)?;
                Ok(serde_json::from_str(&body_str)?)
            }
            code => {
                let body_bytes = hyper::body::to_bytes(resp.into_body()).await?;

                // Convert the body bytes to utf-8
                // let body = String::from_slice(body_bytes.into()).unwrap();
                let body_str = std::str::from_utf8(&body_bytes)?;

                Err(PantryError::ApiError(code, body_str.into()))
            }
        }
    }

    /// Prompts a session, triggering inference by the LLM.
    ///
    /// Requires [UserPermissions::perm_session].
    ///
    /// Session must be running first, call [PantryAPI::create_session],
    /// [PantryAPI::create_session_id], or [PantryAPI::create_session_flex] first.
    ///
    /// # Arguments
    ///
    /// * `user_id` — A UUID, obtained from [PantryAPI::register_user].
    /// * `api_key` — An API key, obtained from [PantryAPI::register_user]
    /// * `session_id` — A UUID representing the session. Obtained by calling
    /// [PantryAPI::create_session] or its variants.
    /// * `llm_uuid` — UUID of the llm. Must match the call used to make the session.
    /// * `prompt` — Prompt for the LLM. Pantry does no preprompting, so if you want a
    /// chatbot style response, you'll need to insert a chatbot type prompt _then_ whatever
    /// the user requested.
    /// * `parameters` — Things like temperature or k value. Whats available varies by LLM,
    /// you can find out what an LLM has either in the UI or in the `user_parameters` and
    /// `user_session_parameters` vectors of an [LLMStatus].
    pub async fn prompt_session_stream(
        &self,
        user_id: Uuid,
        api_key: String,
        session_id: Uuid,
        llm_uuid: String,
        prompt: String,
        parameters: HashMap<String, Value>,
    ) -> Result<LLMEventStream, PantryError> {
        let prompt_session_stream_request = PromptSessionStreamRequest {
            user_id: user_id.to_string(),
            api_key,
            session_id: session_id.to_string(),
            llm_uuid: llm_uuid.to_string(),
            prompt,
            parameters,
        };
        let body = serde_json::to_string(&prompt_session_stream_request)?;

        let resp = self
            .double_edge(hyper::Method::POST, body, format!("/prompt_session_stream"))
            .await?;
        let bod = resp.into_body();

        let stream = decode_stream(TryStreamExt::into_async_read(
            bod.into_stream()
                .map_err(|e| io::Error::new(io::ErrorKind::Other, e)),
        ));

        let events = stream.into_stream().filter_map(|x| async move {
            match x {
                Ok(event) => match event {
                    Event::Retry { retry: _ } => None,
                    Event::Message {
                        id: _,
                        event: _,
                        data,
                    } => {
                        let llm_event: LLMEvent = serde_json::from_str(&data).ok()?;
                        Some(llm_event)
                    }
                },
                Err(e) => {
                    println!("Error: {:?}", e);
                    None
                }
            }
        });
        let out = Box::pin(events);
        // // println!("test2 {:?}", (out.next().into() as LLMEvent));
        // let item_option = out.next().await; // This will give you Option<LLMEvent>
        // match item_option {
        //     Some(item) => println!("test2 {:?}", item),
        //     None => println!("Stream is empty or has ended"),
        // }
        // let item_option = out.next().await; // This will give you Option<LLMEvent>
        // match item_option {
        //     Some(item) => println!("test2 {:?}", item),
        //     None => println!("Stream is empty or has ended"),
        // }

        Ok(out)
    }

    /// Acquire a bare model.
    ///
    /// In practice this means a file path to a GGML file that you can then run yourself
    /// with whatever your preferred LLM runner is.
    ///
    /// Requires [UserPermissions::perm_bare_model].
    ///
    /// # Arguments
    ///
    /// * `user_id` — A UUID, obtained from [PantryAPI::register_user].
    /// * `api_key` — An API key, obtained from [PantryAPI::register_user]
    /// * `llm_id` — UUID of an LLM.
    pub async fn bare_model(
        &self,
        user_id: Uuid,
        api_key: String,
        llm_id: Uuid,
    ) -> Result<BareModelResponse, PantryError> {
        let load_llm_request = BareModelRequest {
            user_id: user_id.to_string(),
            api_key,
            llm_id: llm_id.to_string(),
        };
        let bod = serde_json::to_string(&load_llm_request)?;
        let resp = self
            .double_edge(hyper::Method::POST, bod, format!("/bare_model"))
            .await?;
        match resp.status() {
            StatusCode::OK => {
                // Get the response body bytes.
                let body_bytes = hyper::body::to_bytes(resp.into_body()).await?;

                // Convert the body bytes to utf-8
                // let body = String::from_slice(body_bytes.into()).unwrap();
                let body_str = std::str::from_utf8(&body_bytes)?;
                Ok(serde_json::from_str(&body_str)?)
            }
            code => {
                let body_bytes = hyper::body::to_bytes(resp.into_body()).await?;

                // Convert the body bytes to utf-8
                // let body = String::from_slice(body_bytes.into()).unwrap();
                let body_str = std::str::from_utf8(&body_bytes)?;

                Err(PantryError::ApiError(code, body_str.into()))
            }
        }
    }

    /// Returns a bare model based on filter and preference.
    ///
    /// Requires [UserPermissions::perm_bare_model].
    ///
    /// # Arguments
    ///
    /// * `user_id` — A UUID, obtained from [PantryAPI::register_user].
    /// * `api_key` — An API key, obtained from [PantryAPI::register_user]
    /// * `filter` — A [LLMFilter] object, for what _must_ be true of an LLM to use it.
    /// * `preference` — A [LLMPreference] object, for how to rank and then select from the LLMs
    pub async fn bare_model_flex(
        &self,
        user_id: Uuid,
        api_key: String,
        filter: Option<LLMFilter>,
        preference: Option<LLMPreference>,
    ) -> Result<BareModelResponse, PantryError> {
        let load_llm_request = BareModelFlexRequest {
            user_id: user_id.to_string(),
            api_key,
            filter,
            preference,
        };
        let body = serde_json::to_string(&load_llm_request)?;
        let resp = self
            .double_edge(hyper::Method::POST, body, format!("/bare_model_flex"))
            .await?;
        match resp.status() {
            StatusCode::OK => {
                // Get the response body bytes.
                let body_bytes = hyper::body::to_bytes(resp.into_body()).await?;

                // Convert the body bytes to utf-8
                // let body = String::from_slice(body_bytes.into()).unwrap();
                let body_str = std::str::from_utf8(&body_bytes)?;
                Ok(serde_json::from_str(&body_str)?)
            }
            code => {
                let body_bytes = hyper::body::to_bytes(resp.into_body()).await?;

                // Convert the body bytes to utf-8
                // let body = String::from_slice(body_bytes.into()).unwrap();
                let body_str = std::str::from_utf8(&body_bytes)?;

                Err(PantryError::ApiError(code, body_str.into()))
            }
        }
    }
}
pub type LLMEventStream = Pin<Box<dyn Stream<Item = LLMEvent> + Send>>;

// while let Some(item) = stream.next().await {
//     match item {
//         // Ok(bytes) => {
//         Ok(event) => {
//             // let body_str = std::str::from_utf8(&bytes)?;
//             // println!("Received: {}", body_str);

//             // let string = String::from_utf8_lossy(&bytes);
//             // let mut parser = Parser::new(&string);
//             // let event = parser.next_event();

//             println!("Body Event: {:?}", event);

//             if let Some(event_data) = event.data {
//                 let event: LLMEvent = serde_json::from_str(&event_data)?;
//                 match event.event {
//                     LLMEventInternal::PromptProgress { previous, next } => {
//                         print!("Item: {}", next);
//                     }
//                     _ => {}
//                 }
//             }
//         }
//         Err(e) => println!("Error: {}", e),
//     }
// }
// println!("done");
