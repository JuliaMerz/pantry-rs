use chrono::{DateTime, Utc};
use serde;
use serde_json::Value;
use std::collections::HashMap;
use std::fmt;
use uuid::Uuid;

/*
 * User info returned by the API, exclusively describing the current user.
 * `id` and `api_key` are required to reconstruct the user later.
 * Any permission requests made are attached to this identity.
 */
#[derive(Debug, Clone, serde::Serialize, serde::Deserialize)]
pub struct UserInfo {
    pub id: String,
    // Can be anything, useful for the user to do.
    pub name: String,
    pub api_key: String,

    pub perm_superuser: bool,
    pub perm_load_llm: bool,
    pub perm_unload_llm: bool,
    pub perm_download_llm: bool,
    pub perm_session: bool, //this is for create_sessioon AND prompt_session
    pub perm_request_download: bool,
    pub perm_request_load: bool,
    pub perm_request_unload: bool,
    pub perm_view_llms: bool,
    pub perm_bare_model: bool,
}

/*
 * Represents a capability of an LLM.
 *
 * At the moment, 10 represents GPT-4 quality.
 */
#[derive(serde::Serialize, serde::Deserialize, Clone, Copy, Debug, PartialEq, Eq, Hash)]
#[serde(rename_all = "lowercase")]
pub enum CapabilityType {
    General,
    Assistant,
    Writing,
    Coding,
}

/*
 * Represents a pantry LLM.
 */
#[derive(serde::Deserialize, serde::Serialize, Debug)]
pub struct LLMStatus {
    pub id: String,
    pub family_id: String,
    pub organization: String,

    pub name: String,
    pub homepage: String,
    pub license: String,
    pub description: String,

    pub capabilities: HashMap<CapabilityType, i32>,
    pub requirements: String,
    pub tags: Vec<String>,

    pub url: String,

    pub local: bool,
    pub connector_type: String,
    /*
     * Configuration for connectors. Varies by connector.
     *
     * Currently available keys:
     *
     * *llmrs*
     * - model_architecture (automatically set by UI, but necessary in API)
     * - vocabulary_path [HuggingFaceTokenizerFile](https://github.com/rustformers/llm/blob/2259555544dbf3fadf609b3883f1edda4eb67677/crates/llm-base/src/tokenizer/mod.rs#L56)
     * - vocabulary_repository [HuggingFaceTokenizerString](https://github.com/rustformers/llm/blob/2259555544dbf3fadf609b3883f1edda4eb67677/crates/llm-base/src/tokenizer/mod.rs#L56)
     */
    pub config: HashMap<String, Value>,

    //These aren't _useful_ to the user, but we include them for advanced users
    //to get details.
    pub parameters: HashMap<String, Value>, // Hardcoded Parameters
    pub user_parameters: Vec<String>,       //User Parameters
    pub session_parameters: HashMap<String, Value>, // Hardcoded Parameters
    pub user_session_parameters: Vec<String>, //User Parameters

    //non llminfo fields
    pub uuid: String, // All LLMStatus are downloaded,
    pub running: bool,
}
//This is a lot like frontend::LLMRunningInfo, but limited for non-superusers
#[derive(serde::Deserialize, serde::Serialize, Debug)]
pub struct LLMRunningStatus {
    pub llm_info: LLMStatus,
    pub uuid: String,
    // #[serde(skip_serializing)]
    // pub llm: dyn LLMWrapper + Send + Sync
}

#[derive(Debug, Clone, serde::Serialize, serde::Deserialize)]
pub struct DownloadRequest {
    pub llm_registry_entry: LLMRegistryEntry,
}

#[derive(Debug, Clone, serde::Serialize, serde::Deserialize)]
pub struct PermissionRequest {
    pub requested_permissions: UserPermissions,
}

#[derive(Debug, Clone, serde::Serialize, serde::Deserialize)]
pub struct LoadRequest {
    pub llm_id: String,
}

#[derive(Debug, Clone, serde::Serialize, serde::Deserialize)]
pub struct UnloadRequest {
    pub llm_id: String,
}

#[derive(Debug, Clone, serde::Serialize, serde::Deserialize)]
#[serde(tag = "type")]
pub enum UserRequestType {
    DownloadRequest(DownloadRequest),
    PermissionRequest(PermissionRequest),
    LoadRequest(LoadRequest),
    UnloadRequest(UnloadRequest),
}

#[derive(Debug, serde::Deserialize, serde::Serialize)]
pub struct UserRequestStatus {
    pub id: Uuid,
    pub user_id: Uuid,
    pub timestamp: DateTime<Utc>,
    pub request: UserRequestType,
    pub accepted: bool,
    pub complete: bool,
}

/// Returned by inference, containing inference events.
#[derive(Clone, serde::Deserialize, serde::Serialize, Debug)]
pub struct LLMEvent {
    pub stream_id: Uuid,
    pub timestamp: DateTime<Utc>,
    pub call_timestamp: DateTime<Utc>,
    pub parameters: HashMap<String, Value>,
    pub input: String,
    pub llm_uuid: Uuid,
    pub session: LLMSessionStatus,
    pub event: LLMEventInternal,
}

#[derive(Clone, serde::Deserialize, serde::Serialize, Debug)]
#[serde(tag = "type")]
pub enum LLMEventInternal {
    PromptProgress { previous: String, next: String }, // Next words of an LLM.
    PromptCompletion { previous: String },             // Finished the prompt
    PromptError { message: String },
    Other,
}

/// Structure representing user permissions, generally used for making requests.
///
/// See documentation on [crate::api::PantryAPI] for which calls require which permissions.
#[derive(Debug, Clone, serde::Serialize, serde::Deserialize)]
pub struct UserPermissions {
    // We flatten these in here for easier DB storage.
    pub perm_superuser: bool,
    pub perm_load_llm: bool,
    pub perm_unload_llm: bool,
    pub perm_download_llm: bool,
    pub perm_session: bool, //this is for create_sessioon AND prompt_session
    pub perm_request_download: bool,
    pub perm_request_load: bool,
    pub perm_request_unload: bool,
    pub perm_view_llms: bool,
    pub perm_bare_model: bool,
}

/// This is a minimal copy of session internals returned with [LLMEvent].
#[derive(Debug, Clone, serde::Serialize, serde::Deserialize)]
pub struct LLMSessionStatus {
    pub id: Uuid, //this is a uuid
    pub llm_uuid: Uuid,
    pub user_id: Uuid,
    pub started: DateTime<Utc>,
    pub last_called: DateTime<Utc>,
    pub session_parameters: HashMap<String, Value>,
}

/// Registry entry, containing all the information to upload an LLM.
///
/// Most of this information is non-mandatory, and it's fine to send empty
/// strings. Try to avoid it if possible, since your app's user will see
/// the information in their Pantry UI.
///
/// At present, [LLMConnectorType::LLMrs] is the only working connector,
/// and using it requires config['model_architecture'] to be set according to
/// the [rustformers/llm documentation](https://docs.rs/llm/latest/llm/enum.ModelArchitecture.html)
#[derive(Debug, serde::Serialize, serde::Deserialize, Clone)]
pub struct LLMRegistryEntry {
    pub id: String,
    pub family_id: String,
    pub organization: String,

    pub name: String,
    pub license: String,
    pub description: String,
    pub homepage: String,

    pub capabilities: HashMap<String, i32>,
    pub tags: Vec<String>,
    pub requirements: String,

    /// For security reasons, this gets overwritten by remote and returned
    /// when making either a download request or a download command.
    pub backend_uuid: String,
    pub url: String,

    pub config: HashMap<String, Value>,
    pub local: bool,
    pub connector_type: LLMConnectorType,

    pub parameters: HashMap<String, Value>,
    pub user_parameters: Vec<String>,

    pub session_parameters: HashMap<String, Value>,
    pub user_session_parameters: Vec<String>,
}

#[derive(Debug, serde::Serialize, serde::Deserialize, Clone)]
#[serde(rename_all = "lowercase")]
pub enum LLMConnectorType {
    GenericAPI,
    LLMrs,
    OpenAI,
}

impl fmt::Display for LLMConnectorType {
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        match self {
            LLMConnectorType::GenericAPI => write!(f, "GenericAPI"),
            LLMConnectorType::LLMrs => write!(f, "LLMrs"),
            LLMConnectorType::OpenAI => write!(f, "OpenAI"),
        }
    }
}
