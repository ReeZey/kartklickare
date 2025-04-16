use directories::BaseDirs;
use tauri::Url;

#[cfg_attr(mobile, tauri::mobile_entry_point)]
pub fn run() {
    tauri::Builder::default()
        .setup(|app| {
            let webview_window = tauri::WebviewWindowBuilder::new(
                app, 
                "label", 
                tauri::WebviewUrl::External(
                    Url::parse("https://geoguessr.com/").unwrap()
                )
            )
            .data_directory(BaseDirs::new().unwrap().data_local_dir().join("kartklickare"))
            .build()?;

            //webview_window.open_devtools();
            webview_window.set_title("kartklickare")?;
            Ok(())
        })
        .plugin(tauri_plugin_opener::init())
        .run(tauri::generate_context!())
        .expect("error while running tauri application");
}
