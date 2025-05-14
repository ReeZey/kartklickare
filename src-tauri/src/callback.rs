use std::sync::Arc;

use once_cell::sync::Lazy;
use rand::{distr::Alphanumeric, Rng};
use serde_json::{Map, Value};
use tauri::{Runtime, Webview};
use tokio::sync::{oneshot::Sender, Mutex};
use tokio::sync::oneshot;

static CALLBACK_POOL: Lazy<Arc<Mutex<Vec<CallbackEntry>>>> = Lazy::new(|| {
    Arc::new(Mutex::new(Vec::new()))
});

struct CallbackEntry {
    id: String,
    callback: Option<Sender<Result<Value, String>>>,
}

#[tauri::command]
pub async fn cb(json: Map<String, Value>, id: String) {

    let response_json = json.clone();

    let is_error = json.contains_key("err");

    let mut callbacks = CALLBACK_POOL.lock().await;
    let entry = callbacks.iter_mut().find(|e| e.id == id);

    if let Some(entry) = entry {
        if let Some(callback) = entry.callback.take() {
            if is_error {
                callback.send(Err(response_json.get("err").unwrap().to_string())).unwrap();
            } else {
                callback.send(Ok(Value::Object(response_json))).unwrap();
            }

            callbacks.retain(|e| e.id != id);
        }
    } else {
        println!("No callback found for id: {}", id);
    }
}

pub async fn send_request<R: Runtime>(window: &Webview<R>, url: impl AsRef<str>, extra_json: Option<Value>) -> Result<Value, String> {
    let id = rand::rng()
        .sample_iter(&Alphanumeric)
        .take(32)
        .map(char::from)
        .collect();

    let extra_params = extra_json.unwrap_or(Value::Object(Map::new())).as_object()
        .unwrap_or(&Map::new())
        .iter()
        .map(|(k, v)| format!("response.{} = {};", k, v))
        .collect::<Vec<_>>()
        .join("");
    
    let js = format!(r#"
    async function sendRequest() {{
        try {{
            let request = await fetch('{}', {{
                method: 'GET',
                credentials: 'include',
            }});
            let response = await request.json();

            {}

            return response;
        }} catch (error) {{
            return {{err: error.toString()}};
        }}
    }}

    (async () => {{
        let result = await sendRequest();

        window.__TAURI__.core.invoke('cb', {{ json: result, id: '{}' }});
    }})();
    
    "#, url.as_ref(), extra_params, id);

    window.eval(js).unwrap();

    let (tx, rx) = oneshot::channel::<Result<Value, String>>();

    let mut callbacks = CALLBACK_POOL.lock().await;
    let entry = CallbackEntry {
        id,
        callback: Some(tx),
    };
    callbacks.push(entry);
    drop(callbacks);

    match rx.await {
        Ok(response) => {
            return response;
        },
        Err(err) => {
            return Err(format!("Something went wrong with callback {}", err.to_string()));
        },
    }
}
