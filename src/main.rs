use warp::{Filter, ws};
use futures::{StreamExt, SinkExt};
use serde::{Deserialize, Serialize};


#[derive(Debug, Serialize, Deserialize)]
struct Lyrics {
    time: usize,
    words: Vec<Word>,
}

impl Lyrics {
    fn to_line(&self) -> String {
        let mut line = String::new();
        for word in &self.words {
            line.push_str(&word.string);
            line.push(' ');
        }
        line.trim().to_string()
    }
}

#[derive(Debug, Serialize, Deserialize)]
struct Word {
    string: String,
}

#[derive(Debug, Serialize, Deserialize)]
struct Song {
    lyrics: Option<Vec<Lyrics>>,
}

#[derive(Debug, Deserialize, Serialize)]
struct Time {
    time: usize
}

struct LyricWriter {
    song: Song,
    index: Option<usize>,
}

trait Writer {
    fn set_song(&mut self, song: Song);
    fn output_lyrics(&mut self, time: Time);
}

impl Writer for LyricWriter {
    fn set_song(&mut self, song: Song){
        self.index = None;
        self.song = song;
        println!("")
    }

    fn output_lyrics(&mut self, time: Time) {

        match &self.song.lyrics {
            None => match self.index {
                Some(_) => return,
                None => { println!("No Lyrics found"); self.index = Some(1); return }
            },
            Some(lyrics) => {
                let t = time.time;
                let mut closest_line: Option<&Lyrics> = None;

                // Iterate over each lyric line in the song
                for line in lyrics.iter() {
                    // If the line's time is greater than the given time, exit the loop
                    if line.time > t {
                        break;
                    }
                    // Otherwise, set this line as the closest so far
                    closest_line = Some(line);
                }

                if let Some(line) = closest_line {
                    let current_index = lyrics.iter().position(|lyrics| lyrics.time >= line.time).unwrap_or_default();
                    if self.index != Some(current_index) {
                        self.index = Some(current_index);
                        println!("{}", line.to_line());
                    }
                } 
            }
        }
    }
}

impl LyricWriter {
    fn new() -> Self{
        LyricWriter{
            song: Song { lyrics: None },
            index: None,
        }
    }
}


#[tokio::main]
async fn main() {
    
    let ws_route = ws()
        .map(move |ws: warp::ws::Ws| {
            let mut lyric_writer = LyricWriter::new();
            ws.on_upgrade(move |websocket| {
                async move {
                    if let Err(e) = handle_websocket(websocket, &mut lyric_writer).await {
                        eprintln!("websocket error: {}", e);
                    }
                }
            })
        });
    
    let routes = ws_route.or(warp::any().map(|| "Hello, world!"));
    
    warp::serve(routes).run(([127, 0, 0, 1], 5001)).await;
}

async fn handle_websocket(websocket: warp::ws::WebSocket, lyric_writer: &mut LyricWriter) -> Result<(), Box<dyn std::error::Error>> {
    println!("Client connected");
    let (mut tx, mut rx) = websocket.split();
    
    while let Some(result) = rx.next().await {
        let message = result?;
        let str_message = match message.to_str() {
            Ok(s) => s,
            Err(_) => "{lyrics: null}"
        };
        
        match serde_json::from_str::<Time>(str_message) {
            Ok(time) => lyric_writer.output_lyrics(time),
            Err(_) => match serde_json::from_str::<Song>(str_message){
                Ok(song) => lyric_writer.set_song(song),
                Err(err) => if message.is_close() { tx.close().await? } 
                            else { println!("Unknown message type: {:?}, {:?}", message, err); }
            }
        }
    }
    
    Ok(())
}
