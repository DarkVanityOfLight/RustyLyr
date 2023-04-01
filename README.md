# Lyric Server

This Rust program is a simple web socket server that receivs and shows song lyrics over the network from a client. The server can handle both synced and unsynced lyrics. The program uses the warp library to manage the WebSocket connections.
The server is designed to work with the frontend found at https://github.com/DarkVanityOfLight/lyric-server, which is a Spicetify plugin to send the lyrics.

## Dependencies

This program uses the following Rust crates:

    - **warp** for managing WebSocket connections
    - **futures** for managing asynchronous tasks
    - **serde** for serializing and deserializing data structures
    - **clap** for parsing command line arguments

## Usage

The program is launched with the cargo run command. The following command line arguments are available:

```
Usage: target/debug/lyric-server [OPTIONS]

Options:
  -p, --port <port>              Sets the port the server listens on (default: 5001)
  --output-size <output-size>    Sets the output size of the lyrics (default: None)
  --no-lyrics-message <message>  Sets the message displayed when there are no lyrics available (default: "No lyrics found ;(")
  --debug                        Enables debug mode (default: false)
```


## Building
You can build this into a static binary using `cargo build --release`
A polybar example module can be found at [rustylyr.ini](./rustylyr.ini)


## Code Overview

The server starts by parsing the command line arguments using clap. Then, a ws_route is defined to handle WebSocket connections. A new LyricWriter is created for each WebSocket connection, and this struct handles outputting lyrics to the WebSocket. Finally, the server is started with warp.

The LyricWriter struct contains the current song, the current index of the lyrics, and the message to display when there are no lyrics. The Writer trait is used to output lyrics. The output_lyrics method outputs lyrics to the WebSocket, and the set_song method sets the current song.

The handle_websocket function is called for each WebSocket connection, and it uses the LyricWriter to send lyrics to the client.
