use directories::BaseDirs;
use tauri::Url;
use discord_rich_presence::{activity::{Activity, Timestamps}, DiscordIpc, DiscordIpcClient};

#[cfg_attr(mobile, tauri::mobile_entry_point)]
pub fn run() {
    let mut client = DiscordIpcClient::new("1366798864249786468").unwrap();

    client.connect().unwrap();
    client.set_activity(Activity::new()
        .state("laddar...")
    ).unwrap();
    
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

    println!("Tauri app closed, disconnecting Discord IPC client");
    client.close().unwrap();
}
