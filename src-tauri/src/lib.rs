use directories::BaseDirs;
use tauri::{Manager, Url};
use tokio::sync::Mutex;

mod navigation;
mod callback;

#[cfg_attr(mobile, tauri::mobile_entry_point)]
pub fn run() {
    tauri::Builder::default()
        .setup(|app| {
            app.manage(Mutex::new(navigation::PlayerInfo::default()));
            let webview_window = tauri::WebviewWindowBuilder::new(
                app, 
                "label", 
                tauri::WebviewUrl::External(
                    Url::parse("https://geoguessr.com/").unwrap()
                )
            )
            .data_directory(BaseDirs::new().unwrap().data_local_dir().join("kartklickare"))
            .build()?;
            
            webview_window.open_devtools();
            webview_window.set_title("kartklickare")?;
            Ok(())
        })
        .plugin(tauri_plugin_opener::init())
        .plugin(navigation::init())
        .invoke_handler(tauri::generate_handler![callback::cb,])
        .run(tauri::generate_context!())
        .expect("error while running tauri application");
}
