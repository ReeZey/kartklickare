use std::time::Duration;

use serde_json::Value;
use tauri::{plugin::{Builder, TauriPlugin}, Runtime, Webview};

const _LIVE_API: &str = "https://game-server.geoguessr.com/api";
const OFFLINE_API: &str = "https://www.geoguessr.com/api/v3/games";

#[tauri::command]
pub async fn game_data(message: String) {
    let json: Value = serde_json::from_str(&message).unwrap();

    let round = json["round"].as_u64().unwrap();
    let max_rounds = json["roundCount"].as_u64().unwrap();
    let map_name = json["mapName"].as_str().unwrap();
    let score = json["player"]["totalScore"]["amount"].as_str().unwrap();
    let score_type = json["player"]["totalScore"]["unit"].as_str().unwrap();
    let mode = json["mode"].as_str().unwrap();
    
    println!("Round: {}/{}", round, max_rounds);
    println!("Map name: {}", map_name);
    println!("Score: {} {}", score, score_type);
    println!("Mode: {}", mode);
}


pub fn init<R: Runtime>() -> TauriPlugin<R> {
    Builder::new("navigation")
        .on_webview_ready(|wb | navigation(wb))
        .build()
}

pub fn navigation<R: Runtime>(window: Webview<R>) {
    tauri::async_runtime::spawn( async move {
        let multi_player_games = vec![
            "/live-challenge",
            "/duels",
            "/battle-royale",
            "/bullseye"
        ];

        let mut last_url = window.url().unwrap().path().to_string();
        
        loop {
            let current_url = window.url().unwrap().path().to_string();

            
            let is_live_game = multi_player_games.iter().any(|&game| current_url.contains(game));
            let is_offline_game = current_url.contains("/game");
            
            if is_live_game || is_offline_game {
                if current_url == last_url {
                    let mut new_url = false;
                    for _ in 0..20 {
                        let current_url = window.url().unwrap().path().to_string();
                        if current_url != last_url {
                            new_url = true;
                            break;
                        }

                        tokio::time::sleep(Duration::from_secs(1)).await;
                    }
                    if new_url {
                        continue;
                    }
                }
            }
            if current_url != last_url {
                println!("Current URL: {:?}", current_url);
            }
            last_url = current_url.clone();

            if is_live_game {
                println!("Detected live game");
            } else if is_offline_game {
                println!("Detected offline game");

                let js = format!(r#"
                (async () => {{
                    let request = await fetch('{OFFLINE_API}/{}');
                    let response = await request.text();

                    window.__TAURI__.core.invoke('game_data', {{ 
                        message: response
                    }});
                }})();
                "#, current_url.replace("/game/", ""));

                window.eval(js).unwrap();
            }
            
            tokio::time::sleep(Duration::from_secs(1)).await;
        }
    });
}
