//! Client library for the Pantry LLM API.
//!
//! It's strongly recommended that you use [PantryClient] and [LLMSession], which are a higher
//! level wrapper around [PantryAPI].
//!
//! ```
//! let perms = UserPermissions {
//!     perm_superuser: false,
//!     perm_load_llm: false,
//!     perm_unload_llm: false,
//!     perm_download_llm: false,
//!     perm_session: true, //this is for create_session AND prompt_session
//!     perm_request_download: true,
//!     perm_request_load: true,
//!     perm_request_unload: true,
//!     perm_view_llms: true,
//! };
//!
//! let pantry = PantryClient::register("my project name".into(), perms).await.unwrap();
//!
//! // Pause here and use the UI to accept the permission request.
//!
//! // The empty hashmap means we just use default parameters.
//! // create_session just uses the best currently running LLM. use create_session_id or _flex for
//! // more finegrained control
//! let sess = pantry.create_session(HashMap::new()).await.unwrap();
//!
//! let recv = ses.prompt_session("About me: ".into(), HashMap::new()).await.unwrap();
//! ```
//!
//! If you aren't already running an LLM from the ui, you can use
//! ```
//! pantry.load_llm_flex(None, None).await.unwrap();
//! ```
//!
//! If you want to use your existing ggml infrastructure, you can get a bare model path
//!
//! ```
//! let (model, path) = pantry.bare_model_flex(None, None).await.unwrap();
//! ```
pub use self::error::PantryError;
use self::interface::{
    LLMRegistryEntry, LLMStatus, UserPermissions, UserRequestStatus,
};

pub use api::PantryAPI;
pub use api::{LLMFilter, LLMPreference};


use interface::LLMRunningStatus;
use serde_json::Value;
use std::collections::HashMap;

use uuid::Uuid;

pub mod api;
pub mod error;
pub mod interface;

/// Wrapper around the Pantry LLM API.
///
/// The API client connects to the Pantry application, which by default runs a server on
/// `/tmp/pantrylock.sock` and on `0.0.0.0:9404`. If the pantry application is not running,
/// all api calls will fail.
///
/// Accessing the API requires a `user_id` and an `api_key`. If you don't have those yet,
/// use [PantryClient::register] tog retrieve them and get an instance of the struct. Otherwise,
/// use [PantryClient::login]. Note that, although it is named for convenience, it doesn't
/// actually make any calls to the API—The API key is your sole authentication mechanism, so
/// so store it securely.
///
/// Most capabilities exist either as a request, which the user confirms via the UI, or an
/// automatic "do". If you're writing an application for yourself, we recommend giving yourself
/// broad permissions and going with the "do" option. When writing applications an application
/// for users, we recommend being conservative and submitting a "request" instead of downloading
/// large files to their computer or intensively using resources without asking.
///
/// The same is true for the _id or _flex calls to load and prompt LLMs: Be specific for yourself,
/// and as broad as possible with others.
pub struct PantryClient {
    /// user_id is a UUID representing the remote user
    pub user_id: Uuid,
    pub api_key: String,

    pub client: PantryAPI,
}

impl PantryClient {
    /// Registers a new LLM client.
    ///
    /// Makes two API calls internally: one to create the user, one to request the permissions.
    ///
    /// * `name` — used for debug output and manager display.
    /// * `permissions` — The permissions this api user wants.
    pub async fn register(
        name: String,
        permissions: UserPermissions,
    ) -> Result<(Self, UserRequestStatus), PantryError> {
        let client = PantryAPI {
            client: hyper::Client::new(),
            base_url: "http://localhost:9404/".into(),
        };
        let res = client.register_user(name).await?;

        let user_id =
            Uuid::parse_str(&res.id).map_err(|e| (PantryError::OtherFailure(e.to_string())))?;

        let api = PantryClient {
            user_id: user_id,
            api_key: res.api_key,
            client: client.clone(),
        };

        let res2 = client
            .request_permissions(api.user_id.clone(), api.api_key.clone(), permissions)
            .await?;

        Ok((api, res2))
    }

    // /*
    //  * Registers _without_ requesting any permissions. Note that without
    //  * permissions there's very few things you'll be able to do.
    //  */
    // pub async fn new(name: String) -> Result<Self, PantryError> {
    //     let client = PantryClient {
    //         client: hyper::Client::new(),
    //         base_url: "/".into(),
    //     };
    //     let res = client.register_user(name).await?;

    //     Ok(PantryAPI {
    //         user_id: res.id,
    //         api_key: res.api_key,
    //         client: client,
    //     })
    // }

    /// Creates a [PantryClient] for an existing user.
    ///
    /// Does not make any API calls.
    ///
    /// * `user_id` — A UUID, originally obtained from [PantryClient::register].
    /// * `api_key` — An API key, originally obtained from [PantryClient::register]
    pub fn login(user_id: Uuid, api_key: String) -> Self {
        let client = PantryAPI {
            client: hyper::Client::new(),
            base_url: "/".into(),
        };

        PantryClient {
            user_id,
            api_key,
            client: client,
        }
    }

    /*
     * If the session has been moved to disk, puts it back into memory.
     * Doing this repeatedly for different sessions will result in thrash.
     *
     * Note that the associated LLM _must_ be activated, or Pantry will return
     * an error.
     *
     * TODO: not available until future edition.
     */
    // pub fn load_session_id(&self, session_id: Uuid) -> Result<>{
    //     client.load_session_id(session_id)

    //     todo!();
    // }

    /// Creates a session for an LLM.
    ///
    /// A session is the "state" of a large language model, including its inference history
    /// and its active memory. For a remote LLM this might be effectively nothing.
    /// For a local LLM this call can take some time. Represented by an [LLMSession].
    ///
    /// # Arguments
    ///
    /// * `parameters` — used as session_parameters. Check the UI or an LLMs registry entry
    /// to see which ones are available. Typically most parameters are set at inference time.
    /// Because the function does not know which LLM will be used at call time, Pantry will
    /// _attempt_ to set the given paremeters. The returning [LLMSession] will contain which
    /// parameters, user+system, were actually used to create the session.
    pub async fn create_session(
        &self,
        parameters: HashMap<String, Value>,
    ) -> Result<LLMSession, PantryError> {
        let res = self
            .client
            .create_session(self.user_id.clone(), self.api_key.clone(), parameters)
            .await?;
        let session_uuid = Uuid::parse_str(&res.session_id)
            .map_err(|e| (PantryError::OtherFailure(e.to_string())))?;
        let llm_uuid = Uuid::parse_str(&res.llm_status.uuid)
            .map_err(|e| (PantryError::OtherFailure(e.to_string())))?;

        Ok(LLMSession {
            user_id: self.user_id.clone(),
            api_key: self.api_key.clone(),

            id: session_uuid,
            llm_uuid: llm_uuid,
            session_parameters: res.session_parameters,
            llm_status: res.llm_status,

            client: self.client.clone(),
        })
    }

    /// Creates a session for an LLM.
    ///
    /// A session is the "state" of a large language model, including its inference history
    /// and its active memory. For a remote LLM this might be effectively nothing.
    /// For a local LLM this call can take some time. Represented by an [LLMSession].
    ///
    /// This call will fail if UUID corresponds to an LLM that doesn't exist, or if the
    /// LLM exists but is not currently active/running.
    ///
    /// # Arguments
    ///
    /// * `id` — the UUID of the LLM we wish to use.
    /// * `parameters` — used as session_parameters. Check the UI or an LLMs registry entry
    /// to see which ones are available. Typically most parameters are set at inference time.
    /// Because the function does not know which LLM will be used at call time, Pantry will
    /// _attempt_ to set the given paremeters. The returning [LLMSession] will contain which
    /// parameters, user+system, were actually used to create the session.
    pub async fn create_session_id(
        &self,
        llm_id: Uuid,
        parameters: HashMap<String, Value>,
    ) -> Result<LLMSession, PantryError> {
        let res = self
            .client
            .create_session_id(
                self.user_id.clone(),
                self.api_key.clone(),
                llm_id,
                parameters,
            )
            .await?;
        let session_uuid = Uuid::parse_str(&res.session_id)
            .map_err(|e| (PantryError::OtherFailure(e.to_string())))?;
        let llm_uuid = Uuid::parse_str(&res.llm_status.uuid)
            .map_err(|e| (PantryError::OtherFailure(e.to_string())))?;

        Ok(LLMSession {
            user_id: self.user_id.clone(),
            api_key: self.api_key.clone(),

            id: session_uuid,
            llm_uuid: llm_uuid,
            session_parameters: res.session_parameters,
            llm_status: res.llm_status,

            client: self.client.clone(),
        })
    }

    /// Gets the currently active/running LLMs.
    pub async fn get_running_llms(&self) -> Result<Vec<LLMStatus>, PantryError> {
        let v = self
            .client
            .get_running_llms(self.user_id.clone(), self.api_key.clone())
            .await?;

        Ok(v)
    }

    /// Gets the available LLMs.
    pub async fn get_available_llms(&self) -> Result<Vec<LLMStatus>, PantryError> {
        let v = self
            .client
            .get_available_llms(self.user_id.clone(), self.api_key.clone())
            .await?;

        Ok(v)
    }

    /// Gets a request status
    pub async fn get_request_status(
        &self,
        request_id: Uuid,
    ) -> Result<UserRequestStatus, PantryError> {
        let v = self
            .client
            .get_request_status(self.user_id.clone(), self.api_key.clone(), request_id)
            .await?;

        Ok(v)
    }

    /// Request additional permissions.
    ///
    /// # Arguments
    ///
    /// * `permissions` — The permissions this api user wants.
    pub async fn request_permissions(
        &self,
        perms: UserPermissions,
    ) -> Result<UserRequestStatus, PantryError> {
        self.client
            .request_permissions(self.user_id.clone(), self.api_key.clone(), perms)
            .await
    }

    /// Creates a request to download a new model. Must be accepted by the system
    /// owner (currently via the UI).
    ///
    /// # Arguments
    ///
    /// * `llm_registry_entry` — A valid LLM registry entry to download. This specifies
    /// the location of the model as well as any metadata. For better usability, try
    /// being comprehensive about this.
    pub async fn request_download_llm(
        &self,
        reg: LLMRegistryEntry,
    ) -> Result<UserRequestStatus, PantryError> {
        self.client
            .request_download(self.user_id.clone(), self.api_key.clone(), reg)
            .await
    }

    pub async fn request_load_llm(&self, llm_uuid: Uuid) -> Result<UserRequestStatus, PantryError> {
        self.client
            .request_load(self.user_id.clone(), self.api_key.clone(), llm_uuid)
            .await
    }

    /// Requests a load, but doesn't predetermine the exact LLM ahead of time.
    ///
    /// # Arguments
    ///
    /// * `filter` — An [LLMFilter] specifying hard requirements for the LLM.
    /// * `preference` — An [LLMPreference] specifying soft requirements for the LLM.
    pub async fn request_load_llm_flex(
        &self,
        filter: Option<LLMFilter>,
        preference: Option<LLMPreference>,
    ) -> Result<UserRequestStatus, PantryError> {
        self.client
            .request_load_flex(
                self.user_id.clone(),
                self.api_key.clone(),
                filter,
                preference,
            )
            .await
    }

    /// Requests an LLM be shutdown, conserving resources. This should
    /// generally be non-destructive—Pantry attempts to save sessions to disc on shutdown.
    ///
    /// # Arguments
    ///
    /// * `llm_id` — UUID of the LLM. Find running llms via [PantryClient::get_running_llms].
    pub async fn request_unload_llm(
        &self,
        llm_uuid: Uuid,
    ) -> Result<UserRequestStatus, PantryError> {
        self.client
            .request_unload(self.user_id.clone(), self.api_key.clone(), llm_uuid)
            .await
    }
    /// Requests Pantry to load a specific LLM.
    ///
    /// # Arguments
    ///
    /// * `llm_id` — A UUID for the LLM you want to load. Find one via [PantryClient::get_available_llms].
    pub async fn load_llm(&self, llm_uuid: Uuid) -> Result<LLMRunningStatus, PantryError> {
        self.client
            .load_llm(self.user_id.clone(), self.api_key.clone(), llm_uuid)
            .await
    }

    /// Loads an LLM.
    ///
    /// Requires the [UserPermissions::perm_load_llm] permission.
    ///
    /// # Arguments
    ///
    /// * `filter` — A [LLMFilter] object, for what _must_ be true of an LLM to load it.
    /// * `preference` — A [LLMPreference] object, for how to rank and then select from the LLMs
    /// that pass the filter.
    pub async fn load_llm_flex(
        &self,
        filter: Option<LLMFilter>,
        preference: Option<LLMPreference>,
    ) -> Result<LLMRunningStatus, PantryError> {
        self.client
            .load_llm_flex(
                self.user_id.clone(),
                self.api_key.clone(),
                filter,
                preference,
            )
            .await
    }

    pub async fn unload_llm(&self, llm_uuid: Uuid) -> Result<LLMStatus, PantryError> {
        self.client
            .unload_llm(self.user_id.clone(), self.api_key.clone(), llm_uuid)
            .await
    }

    pub async fn bare_model_flex(
        &self,
        filter: Option<LLMFilter>,
        preference: Option<LLMPreference>,
    ) -> Result<(LLMStatus, String), PantryError> {
        let resp = self
            .client
            .bare_model_flex(
                self.user_id.clone(),
                self.api_key.clone(),
                filter,
                preference,
            )
            .await?;
        Ok((resp.model, resp.path))
    }

    pub async fn bare_model(&self, llm_id: Uuid) -> Result<(LLMStatus, String), PantryError> {
        let resp = self
            .client
            .bare_model(self.user_id.clone(), self.api_key.clone(), llm_id)
            .await?;
        Ok((resp.model, resp.path))
    }
}

pub struct LLMSession {
    pub user_id: Uuid,
    pub api_key: String,

    pub id: Uuid,
    pub llm_uuid: Uuid,
    pub session_parameters: HashMap<String, Value>,
    pub llm_status: LLMStatus,

    pub client: PantryAPI,
}

impl LLMSession {
    /// Prompts a session, triggering inference by the LLM.
    ///
    /// Requires [UserPermissions::perm_session].
    ///
    /// # Arguments
    ///
    /// * `prompt` — Prompt for the LLM. Pantry does no preprompting, so if you want a
    /// chatbot style response, you'll need to insert a chatbot type prompt _then_ whatever
    /// the user requested.
    /// * `parameters` — Things like temperature or k value. Whats available varies by LLM,
    /// you can find out what an LLM has either in the UI or in the `user_parameters` and
    /// `user_session_parameters` vectors of an [LLMStatus].
    pub async fn prompt_session(
        &self,
        prompt: String,
        parameters: HashMap<String, Value>,
    ) -> Result<api::LLMEventStream, PantryError> {
        self.client
            .prompt_session_stream(
                self.user_id.clone(),
                self.api_key.clone(),
                self.id.clone(),
                self.llm_status.uuid.clone(),
                prompt,
                parameters,
            )
            .await
    }

    /// Interrupts ongoing inference.
    ///
    /// Internally this uses a cancellation callback to cancel inference _after the next token_.
    /// Depending on your system, this might take a moment, especially given that some
    /// tokens might already be inferred but not yet transmitted.
    pub async fn interrupt_session(&self) -> Result<LLMRunningStatus, PantryError> {
        let llm_uuid = Uuid::parse_str(&self.llm_status.uuid).unwrap();
        self.client
            .interrupt_session(
                self.user_id.clone(),
                self.api_key.clone(),
                llm_uuid,
                self.id.clone(),
            )
            .await
    }
}
