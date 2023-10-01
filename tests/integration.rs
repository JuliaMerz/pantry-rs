use futures::stream::StreamExt;
use maplit::hashmap;
use pantry_rs::interface::{LLMConnectorType, LLMRegistryEntry, UserPermissions};
use pantry_rs::PantryClient;
use uuid::Uuid;

use std::collections::HashMap;
use std::{thread, time};

#[tokio::test]
async fn basic_workflow() {
    let perms = UserPermissions {
        perm_superuser: false,
        perm_load_llm: true,
        perm_unload_llm: false,
        perm_download_llm: true,
        perm_session: true, //this is for create_session AND prompt_session
        perm_request_download: true,
        perm_request_load: true,
        perm_request_unload: true,
        perm_view_llms: true,
        perm_bare_model: true,
    };

    let (pantry, mut req_status) = PantryClient::register("testing".into(), perms, None)
        .await
        .unwrap();

    //wait for permission requests to be fulfilled.
    //

    let mut timeout_counter = 120;

    while timeout_counter > 0 {
        req_status = pantry.get_request_status(req_status.id).await.unwrap();
        if req_status.complete && req_status.accepted {
            break;
        }
        timeout_counter = timeout_counter - 1;
        thread::sleep(time::Duration::from_secs(1));
    }

    println!("Request accepted, continuing");
    //We need at least one LLM.
    // aw!(pantry.load_llm_flex(None, None)).unwrap();
    let reg = LLMRegistryEntry {
            id: "openchat-3".into(),
            family_id: "llama".into(),
            organization: "openchat".into(),
            name: "Openchat LLM".into(),
            license: "llama2".into(),
            description: "openchat llm".into(),
            homepage: "".into(),
            capabilities: hashmap! {
            "assistant".into() => -1,
            "coding".into() => -1,
            "general".into() => -1,
            "writing".into() => -1
            },
            tags: Vec::new(),
            requirements: "".into(),
            backend_uuid: Uuid::new_v4().to_string(),
            url: "https://huggingface.co/TheBloke/OpenChat_v3.2-GGML/resolve/main/openchat_v3.2.ggmlv3.q4_0.bin".into(),
            config: hashmap! {
                "model_architecture".into() => "llama".into(),
            },
            local: true,
            connector_type: LLMConnectorType::LLMrs,
            parameters: hashmap! {},
            user_parameters: vec![
                "sampler_string".into(),
                "pre_prompt".into(),
                "post_prompt".into(),
            ],
            session_parameters: hashmap! {},
            user_session_parameters: vec!["system_prompt".into()],
        };
    let id = pantry.download_llm(reg).await.unwrap();
    println!("uuid {:?}", id);
    pantry
        .await_download(id, |x| println!("Progress: {:?}", x))
        .await
        .unwrap();

    pantry.load_llm_flex(None, None).await.unwrap();

    let sess = pantry.create_session(HashMap::new()).await.unwrap();

    println!("Running");
    let mut res = sess
        .prompt_session("About me: ".into(), HashMap::new())
        .await
        .unwrap();
    while let Some(event) = res.next().await {
        println!("Got event! {:?}", event);
    }
    println!("!complete???");
}

#[tokio::test]
async fn bare_model_workflow() {
    let perms = UserPermissions {
        perm_superuser: false,
        perm_load_llm: false,
        perm_unload_llm: false,
        perm_download_llm: true,
        perm_session: true, //this is for create_session AND prompt_session
        perm_request_download: true,
        perm_request_load: true,
        perm_request_unload: true,
        perm_view_llms: true,
        perm_bare_model: true,
    };

    let (pantry, mut req_status) = PantryClient::register(
        "bare_model_test".into(),
        perms,
        Some("http://localhost:9404".into()),
    )
    .await
    .unwrap();

    //wait for permission requests to be fulfilled.

    let mut timeout_counter = 120;

    while timeout_counter > 0 {
        req_status = pantry.get_request_status(req_status.id).await.unwrap();
        if req_status.complete && req_status.accepted {
            break;
        }
        timeout_counter = timeout_counter - 1;
        thread::sleep(time::Duration::from_secs(1));
    }

    println!("Request accepted, continuing");

    let (_model, path) = pantry.bare_model_flex(None, None).await.unwrap();
    println!("lol {}", path);
}
//             .route("/register_user", post(register_user))
//             .route("/request_permissions", post(request_permissions))
//             .route("/request_download", post(request_download))
//             .route("/request_load", post(request_load))
//             .route("/request_unload", post(request_unload))
//             .route("/load_llm", post(load_llm))
//             .route("/load_llm_flex", post(load_llm_flex))
//             .route("/unload_llm", post(unload_llm))
//             .route("/download_llm", post(download_llm))
//             .route("/create_session", post(create_session))
//             .route("/create_session_id", post(create_session_id))
//             .route("/create_session_flex", post(create_session_flex))
//             .route("/prompt_session_stream", post(prompt_session_stream))
//             .route("/request_load_flex", post(request_load))
//             .route("/get_llm_status", post(get_llm_status))
//             .route("/get_available_llms", post(get_available_llms))
//             .route("/get_running_llms", post(get_running_llms))
//             .route("/interrupt_session", post(interrupt_session))
