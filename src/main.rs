use warp::{Filter, ws};
use futures::{StreamExt};
use serde::{Deserialize, Serialize};

#[derive(Debug, Deserialize, Serialize)]
struct Word {
    string: String,
}

#[derive(Debug, Deserialize, Serialize)]
struct Lyrics {
    time: i32,
    words: Vec<Word>,
}

#[derive(Debug, Deserialize, Serialize)]
struct Song {
    lyrics: Vec<Lyrics>,
}

#[derive(Debug, Deserialize, Serialize)]
struct Time {
    time: i32
}

#[tokio::main]
async fn main() {
    let ws_route = ws()
        .map(|ws: warp::ws::Ws| {
            ws.on_upgrade(|websocket| async move {
                if let Err(e) = handle_websocket(websocket).await {
                    eprintln!("websocket error: {}", e);
                }
            })
        });
    
    let routes = ws_route.or(warp::any().map(|| "Hello, world!"));
    
    warp::serve(routes).run(([127, 0, 0, 1], 5001)).await;
}

async fn handle_websocket(websocket: warp::ws::WebSocket) -> Result<(), Box<dyn std::error::Error>> {
    println!("Client connected");
    let (_, mut rx) = websocket.split();
    
    while let Some(result) = rx.next().await {
        let message = result?;
        match serde_json::from_str::<Song>(message.to_str().unwrap()){
            Ok(song) => println!("{:?}", song),
            Err(_) => match serde_json::from_str::<Time>(message.to_str().unwrap()) {
                Ok(time) => println!("{:?}", time),
                Err(_) => println!("Unknown message type"),
            }
        }
    }
    
    Ok(())
}
