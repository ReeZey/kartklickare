use std::time::Duration;

use discord_rich_presence::{activity::Activity, DiscordIpc, DiscordIpcClient};
use serde_json::{Map, Value};
use tauri::{plugin::{Builder, TauriPlugin}, Manager, Runtime, Webview};
use tokio::sync::Mutex;

use crate::callback;

const LIVE_API: &str = "https://game-server.geoguessr.com/api";
const OFFLINE_API: &str = "https://www.geoguessr.com/api/v3/games";
const PLAYER_API: &str = "https://www.geoguessr.com/api/v3/profiles/";
const MAP_API: &str = "https://www.geoguessr.com/api/maps";

#[derive(Default)]
pub struct PlayerInfo {
    pub player_name: String,
    pub player_id: String,

    pub discord_client: Option<DiscordIpcClient>,
}

pub fn init<R: Runtime>() -> TauriPlugin<R> {
    Builder::new("navigation")
        .on_webview_ready(|wb | navigation(wb))
        .build()
}

pub fn navigation<R: Runtime>(window: Webview<R>) {
    tauri::async_runtime::spawn( async move {

        let response: Value = callback::send_request(&window, PLAYER_API, None).await.unwrap();
        setup_user(&window, response).await;

        let multi_player_games = vec![
            "/live-challenge",
            "/duels",
            "/battle-royale",
            "/bullseye",
            "/team-duels",
        ];

        let mut last_path = window.url().unwrap().path().to_string();
        
        loop {
            let mut current_path = window.url().unwrap().path().to_string();

            
            let is_live_game = multi_player_games.iter().any(|&game| current_path.starts_with(game));
            let is_offline_game = current_path.starts_with("/game");
            
            if is_live_game || is_offline_game {
                if current_path == last_path {
                    let mut new_url = false;
                    for _ in 0..20 {
                        let current_url = window.url().unwrap().path().to_string();
                        if current_url != last_path {
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
            if current_path != last_path {
                println!("Current URL: {:?}", current_path);
            }
            last_path = current_path.clone();

            if is_live_game {
                println!("Detected live game");

                let game_mode = current_path.split("/").nth(1).unwrap()
                    .split("-").map(|f| f.to_string()).collect::<Vec<String>>()
                    .iter_mut().map(|f| f.remove(0).to_uppercase().to_string() + f)
                    .collect::<Vec<String>>().join(" ");
                
                if current_path.contains("/team-") {
                    current_path = current_path.replace("team-", "");
                }

                let mut extra = Map::new();
                extra.insert("game_type".to_string(), Value::String("live".to_string()));
                extra.insert("game_mode".to_string(), Value::String(game_mode));

                let json = callback::send_request(
                    &window, 
                    &format!("{}/{}", LIVE_API, current_path), 
                    Some(Value::Object(extra))
                ).await.unwrap();

                handle_game_data(&window, json).await;
            } else if is_offline_game {
                println!("Detected offline game");

                let mut extra = Map::new();
                extra.insert("game_type".to_string(), Value::String("offline".to_string()));

                let json = callback::send_request(
                    &window, 
                    &format!("{}/{}", OFFLINE_API, current_path.replace("/game/", "")), 
                    Some(Value::Object(extra))
                ).await.unwrap();

                handle_game_data(&window, json).await;
            } else {
                let state = window.state::<Mutex<PlayerInfo>>();
                let mut player_info = state.lock().await;

                let lookup_urls = [
                    ("/", "In Menu", true),
                    ("/me", "Profile", true),
                    ("/maps", "Browsing maps", true),
                    ("/shop", "Shopping!", false),
                    ("/singleplayer", "Campaign", false),
                    ("/multiplayer", "Looking for game...", true),
                    ("/party", "In Lobby", false),
                    ("/quiz", "Quiz time!", false),
                    ("/competitive-streak", "City streak", false)
                ];

                let (key, mut title_text, print_path) = lookup_urls.iter().rev().find(|(prefix, _, _)| current_path.starts_with(prefix)).map(|f| (f.0, f.1, f.2)).unwrap();
                
                if key.starts_with("/maps") {
                    let json = callback::send_request(
                        &window, 
                        &format!("{}/{}", MAP_API, current_path.replace("/maps/", "")), 
                        None
                    ).await.unwrap();

                    current_path = json["name"].as_str().unwrap().to_string();
                    title_text = "Looking at map";
                }

                let mut activity: Activity = Activity::new().details(title_text);

                if print_path {
                    activity = activity.state(&current_path);
                }

                let discord = player_info.discord_client.as_mut().unwrap();
                discord.set_activity(activity).unwrap();
            }
            
            tokio::time::sleep(Duration::from_secs(1)).await;
        }
    });
}

async fn setup_user<R: Runtime>(window: &Webview<R>, json: Value) {
    let state = window.state::<Mutex<PlayerInfo>>();

    let mut player_info = state.lock().await;

    player_info.player_name = json["user"]["nick"].as_str().unwrap().to_string();
    player_info.player_id = json["user"]["id"].as_str().unwrap().to_string();

    let mut client = DiscordIpcClient::new("1366798864249786468").unwrap();

    client.connect().unwrap();
    client.set_activity(Activity::new()
        .details("Just started!")
    ).unwrap();

    player_info.discord_client = Some(client);
}

pub async fn handle_game_data<R: Runtime>(window: &Webview<R>, json: Value) {
    //println!("Game data: {:?}", json);

    let current_round: u64;
    let max_rounds: u64;
    let mode: &str;
    let map_name: &str;
    let total_score: u64;
    let game_type;

    match json["game_type"].as_str() {
        Some("offline") => {
            current_round = json["round"].as_u64().unwrap();
            max_rounds = json["roundCount"].as_u64().unwrap();
            mode = json["mode"].as_str().unwrap();
            map_name = json["mapName"].as_str().unwrap();
            total_score = json["player"]["totalScore"]["amount"].as_str().unwrap().parse().unwrap();
            game_type = json["game_type"].as_str().unwrap();
        },
        Some("live") => {
            current_round = json["currentRoundNumber"].as_u64().unwrap();
            mode = json["game_mode"].as_str().unwrap();
            if json.get("aggregatedAnswerStats") != None {
                game_type = "Quiz";
            } else {
                game_type = json["game_type"].as_str().unwrap();
            }
            if json.get("options") == None {
                map_name = "Battle Royale";
            } else {
                map_name = json["options"]["map"]["name"].as_str().unwrap();
            }
            max_rounds = 0;
            total_score = 0;
        }
        _ => {
            println!("Unknown game type");
            return;
        }
    }
    
    let mut line1;
    let mut line2 ;

    if mode == "streak" {
        line1 = format!("Country Streak - {}", map_name);
        line2 = format!("Streak: {}", current_round - 1);
    } else {
        line1 = String::new();
        if game_type == "Quiz" {
            line1.push_str(game_type);
        } else if game_type == "live" {
            line1.push_str(&format!("{} - {}", mode, map_name));
        } else {
            line1.push_str(map_name);
        }

        line2 = format!("Round: {}", current_round.to_string());
        
        if max_rounds > 0 {
            line2.push_str(&format!(" / {}", max_rounds));
        }

        if total_score > 0 {
            line2.push_str(&format!(" - {} points", total_score));
        }
    }

    let state = window.state::<Mutex<PlayerInfo>>();
    let mut player_info = state.lock().await;

    let client = player_info.discord_client.as_mut().unwrap();
    client.set_activity(Activity::new()
        .details(&line1)
        .state(&line2)
    ).unwrap();

    println!("{}", line1);
    println!("{}", line2);
}