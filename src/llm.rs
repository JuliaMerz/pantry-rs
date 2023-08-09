use uuid::Uuid;
use crate::api::LLMStatus

pub struct LLM {
    // Machine Info
    pub uuid: Uuid,
    pub id: String, // Maybe?: https://github.com/alexanderatallah/window.ai/blob/main/packages/lib/README.md
    pub family_id: String, // Whole sets of models: Example's are GPT, LLaMA
    pub organization: String, // May be "None"

    // Human Info
    pub name: String,
    pub homepage: String,
    pub description: String,
    pub license: String,

    // Fields only in LLM Available
    pub downloaded_reason: String,
    pub downloaded_date: DateTime<Utc>,
    pub last_called: Option<DateTime<Utc>>,

    // 0 is not capable, -1 is not evaluated.
    pub capabilities: HashMap<String, i32>,
    pub tags: Vec<String>,

    pub url: String,

    // Functionality
    pub local: bool,                                  // Is it not an API connector?
    // Configs used by the connector for setup.
    pub config: HashMap<String, Value>, //Configs used by the connector

    // Parameters — these are submitted at call time.
    // these are the same, except one is configurable by users (programs or direct).
    // Hard coded parameters exist so repositories can deploy simple user friendly models
    // with preset configurations.
    pub parameters: HashMap<String, Value>,          // Hardcoded Parameters
    pub user_parameters: DbVec<String>, //User Parameters

    //These are the same, but for whole sessions.
    //This is largely forward thinking, the only place we would implement
    //this now would be useGPU.
    //But we'll need ot eventually.
    pub session_parameters: HashMap<String, Value>, // Hardcoded Parameters
    pub user_session_parameters: Vec<String>,
}


impl From<&LLMStatus> for LLM {
    fn from(llm: &LLM) -> Self {

    let new_llm: llm::LLM = llm::LLM {
        id: llm.id.clone(),
        family_id: llm.family_id.clone(),
        organization: llm.organization.clone(),
        name: llm.name.clone(),
        license: llm.license.clone(),
        description: llm.description.clone(),
        downloaded_date: chrono::offset::Utc::now(),
        url: llm.url.clone(),
        homepage: llm.homepage.clone(),

        uuid: Uuid::parse_str(uuid),

        capabilities: DbHashMapInt(llm.capabilities.clone()),
        tags: DbVec(llm.tags.clone()),

        requirements: llm.requirements.clone(),

        local: llm.local.clone(),
        connector_type: llm.connector_type.clone(), // assuming this type is also Clone
        config: DbHashMap(llm.config.clone()),
        parameters: DbHashMap(llm.parameters.clone()),
        user_parameters: DbVec(llm.user_parameters.clone()),
        session_parameters: DbHashMap(llm.session_parameters.clone()),
        user_session_parameters: DbVec(llm.user_session_parameters.clone()),
        model_path: DbOptionPathbuf(Some(path.clone())),
    };
}

pub struct LLMSession

pub struct LLMHistoryItem

pub struct User

pub struct Request

pub struct LLMRegistryEntry

