# pantry.rs

Rust library for the [pantry llm runner](https://github.com/JuliaMerz/pantry).

Read the documentation at [docs.rs](https://docs.rs/pantry-rs/latest/pantry_rs/)

## Usage

The API connects to your local pantry installation, so you need to download pantry first.
Client library for the Pantry LLM API.

### Example

It's strongly recommended that you use [PantryClient] and [LLMSession], which are a higher
level wrapper around [PantryAPI].

```
let perms = UserPermissions {
    perm_superuser: false,
    perm_load_llm: false,
    perm_unload_llm: false,
    perm_download_llm: false,
    perm_session: true, //this is for create_session AND prompt_session
    perm_request_download: true,
    perm_request_load: true,
    perm_request_unload: true,
    perm_view_llms: true,
};

let pantry = PantryClient::register("my project name".into(), perms).await.unwrap();

// Pause here and use the UI to accept the permission request.

// The empty hashmap means we just use default parameters.
// create_session just uses the best currently running LLM. use create_session_id or _flex for
// more finegrained control
let sess = pantry.create_session(HashMap::new()).await.unwrap();

let recv = ses.prompt_session("About me: ".into(), HashMap::new()).await.unwrap();
```

If you aren't already running an LLM from the ui, you can use
```
pantry.load_llm_flex(None, None).await.unwrap();
```

If you want to use your existing ggml infrastructure, you can get a bare model path

```
let (model, path) = pantry.bare_model_flex(None, None).await.unwrap();
```
