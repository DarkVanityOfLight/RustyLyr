use clap::Parser;
use futures::{SinkExt, StreamExt};
use serde::{Deserialize, Serialize};
use warp::{ws, Filter};

#[derive(Debug, Serialize, Deserialize)]
struct Lyrics {
    time: usize,
    words: Vec<Word>,
}

#[derive(Debug, Serialize, Deserialize)]
struct UnsyncedLyrics {
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

#[derive(Debug, Serialize, Deserialize)]
struct UnsyncedSong {
    lyrics: Vec<UnsyncedLyrics>,
}

#[derive(Debug, Deserialize, Serialize)]
struct Time {
    time: usize,
}

struct LyricWriter {
    song: SongFormat,
    index: Option<usize>,
    no_lyrics_message: String,
    output_size: Option<usize>,
}

trait Writer {
    fn set_song(&mut self, song: SongFormat);
    fn output_lyrics(&mut self, time: Time);
}

fn pad_or_trim_string(s: &str, length: usize) -> String {
    let s_len = s.chars().count();
    if s_len >= length {
        // Trim the string
        s.chars().take(length).collect()
    } else {
        // Pad the string
        let padding = length - s_len;
        let pad_left = padding / 2;
        let pad_right = padding - pad_left;
        let left_padding = " ".repeat(pad_left);
        let right_padding = " ".repeat(pad_right);
        format!("{}{}{}", left_padding, s, right_padding)
    }
}

enum SongFormat {
    Unsynced(UnsyncedSong),
    Synced(Song),
}

fn find_lyric_line(lyrics: &Vec<Lyrics>, time: Time) -> Option<usize> {
    let mut left = 0;
    let mut right = lyrics.len() - 1;

    while left < right {
        let mid = left + (right - left) / 2;
        if lyrics[mid].time < time.time {
            left = mid + 1;
        } else {
            right = mid;
        }
    }

    if left > 0 {
        Some(left - 1)
    } else {
        None
    }
}

#[test]
fn test_binary_search() {
    let mut time = Time { time: 10 };

    let mut lyrics: Vec<Lyrics> = Vec::new();
    for i in 0..=20 {
        lyrics.push(Lyrics {
            time: i,
            words: Vec::new(),
        })
    }

    println!("{:?}", lyrics);

    assert_eq!(find_lyric_line(&lyrics, time), Some(9));
    time = Time { time: 0 };
    assert_eq!(find_lyric_line(&lyrics, time), None);
    time = Time { time: 21 };
    assert_eq!(find_lyric_line(&lyrics, time), Some(19));
    time = Time { time: 22 };
    assert_eq!(find_lyric_line(&lyrics, time), Some(19));
}

impl Writer for LyricWriter {
    fn set_song(&mut self, song: SongFormat) {
        self.index = None;
        self.song = song;
        println!("")
    }

    fn output_lyrics(&mut self, time: Time) {
        match &self.song {
            SongFormat::Unsynced(UnsyncedSong { lyrics: _ }) => match self.index {
                None => {
                    println!("This song is unsynced :(");
                    self.index = Some(1);
                    return;
                }
                Some(_) => return,
            },
            SongFormat::Synced(Song { lyrics }) => match lyrics {
                None => match self.index {
                    Some(_) => return,
                    None => {
                        println!("{}", self.no_lyrics_message);
                        self.index = Some(1);
                        return;
                    }
                },
                Some(lyrics) => {
                    // Find the closest line
                    let current_index = find_lyric_line(&lyrics, time);
                    let line = match current_index {
                        Some(index) => Some(lyrics.get(index).unwrap()),
                        None => None,
                    };

                    // If we have a different line then before
                    if self.index != current_index {
                        self.index = current_index;

                        // Unwrap the line to a string
                        let string_line = match line {
                            Some(l) => l.to_line(),
                            None => String::from("󰎈"),
                        };

                        // Trim the line if we have to aka if output_size is set
                        let beauty_line = match self.output_size {
                            None => string_line,
                            Some(size) => pad_or_trim_string(&string_line, size),
                        };
                        // Output the line
                        println!("{}", beauty_line);
                    }
                }
            },
        }
    }
}

impl LyricWriter {
    fn new(output_size: Option<usize>, no_lyrics_message: Option<String>) -> Self {
        LyricWriter {
            song: SongFormat::Synced(Song { lyrics: None }),
            index: None,
            output_size: output_size,
            no_lyrics_message: no_lyrics_message.unwrap_or("No Lyrics found ;(".to_string()),
        }
    }
}

#[derive(Parser)]
#[command(author, version, about, long_about = None)]
struct Cli {
    #[arg(short, long, default_value_t = 5001)]
    port: u16,
    output_size: Option<usize>,
    no_lyrics_message: Option<String>,
    #[arg(long, default_value_t = false)]
    debug: bool,
}

#[tokio::main]
async fn main() {
    let cli = Cli::parse();
    let ws_route = ws().map(move |ws: warp::ws::Ws| {
        let output_size = cli.output_size;
        let no_lyrics_message = cli.no_lyrics_message.clone();
        let mut lyric_writer = LyricWriter::new(output_size, no_lyrics_message);
        ws.on_upgrade(move |websocket| async move {
            if let Err(e) = handle_websocket(websocket, &mut lyric_writer).await {
                if cli.debug {
                    eprintln!("websocket error: {}", e);
                }
            }
        })
    });

    let routes = ws_route;

    warp::serve(routes).run(([127, 0, 0, 1], cli.port)).await;
}

async fn handle_websocket(
    websocket: warp::ws::WebSocket,
    lyric_writer: &mut LyricWriter,
) -> Result<(), Box<dyn std::error::Error>> {
    println!("Client connected");
    let (mut tx, mut rx) = websocket.split();

    while let Some(result) = rx.next().await {
        let message = result?;
        let str_message = match message.to_str() {
            Ok(s) => s,
            Err(_) => "{lyrics: null}",
        };

        match serde_json::from_str::<Time>(str_message) {
            Ok(time) => lyric_writer.output_lyrics(time),
            Err(_) => match serde_json::from_str::<Song>(str_message) {
                Ok(song) => lyric_writer.set_song(SongFormat::Synced(song)),
                Err(_) => {
                    if message.is_close() {
                        tx.close().await?
                    } else {
                        match serde_json::from_str::<UnsyncedSong>(str_message) {
                            Ok(song) => lyric_writer.set_song(SongFormat::Unsynced(song)),
                            Err(err) => println!("Unknown message type: {:?}, {:?}", message, err),
                        }
                    }
                }
            },
        }
    }

    Ok(())
}
