use tokio::net::TcpStream;
use tokio::net::TcpListener;

use tokio::sync::{broadcast, mpsc};
use std::collections::HashMap;
use std::sync::{Arc, Mutex};
use std::time::Duration;

use async_bincode::tokio::{AsyncBincodeReader, AsyncBincodeWriter};
use futures::{SinkExt, StreamExt};

use my_redis::{GameState, ServerMessage, ClientMessage};

type Db = Arc<Mutex<GameState>>;

#[tokio::main]
async fn main() {
    let (broadcast_tx, _rx) = broadcast::channel(100);
    let state: Db = Arc::new(Mutex::new(HashMap::new()));

    // Channel for client messages back to main
    let (client_msg_tx, mut client_msg_rx) = mpsc::unbounded_channel::<ClientMessage>();

    // Task to process client messages and update state
    let game_tx = broadcast_tx.clone();
    let update_state = state.clone();
    tokio::spawn(async move {
        while let Some(msg) = client_msg_rx.recv().await {
            match msg {
                ClientMessage::Set(key, value) => {
                    update_state.lock().unwrap().insert(key, value);
                    // ONLY broadcast state AFTER processing client messages
                    let updated_state = update_state.lock().unwrap().clone();
                    let _ = game_tx.send(ServerMessage::GameState(updated_state));
                }
            }
        }
    });

    // heartbeat broadcast
    let game_tx = broadcast_tx.clone();
    let game_state = state.clone();
    tokio::spawn(async move {
        let mut interval = tokio::time::interval(Duration::from_secs(5));
        loop {
            interval.tick().await;
            let state = game_state.lock().unwrap();
            let _ = game_tx.send(ServerMessage::GameState(state.clone()));
        }
    });

    // Accept clients
    let listener = TcpListener::bind("127.0.0.1:8080").await.unwrap();
    println!("Server listening on 127.0.0.1:8080");
    loop {
        let (socket, _) = listener.accept().await.unwrap();
        println!("New client connected");
        let rx = broadcast_tx.subscribe();
        let tx = client_msg_tx.clone();
        tokio::spawn(handle_client(socket, rx, tx));
    }
}

async fn handle_client(
    socket: TcpStream,
    mut rx: broadcast::Receiver<ServerMessage>,
    tx: mpsc::UnboundedSender<ClientMessage>,
) {
    let (reader, writer) = socket.into_split();

    // Wrap with async-bincode for automatic serialization
    use async_bincode::AsyncDestination;
    let mut framed_reader: AsyncBincodeReader<_, ClientMessage> = AsyncBincodeReader::from(reader);
    let mut framed_writer: AsyncBincodeWriter<_, ServerMessage, AsyncDestination> = AsyncBincodeWriter::from(writer).for_async();

    // Task 1: Read from client, send to channel
    let read_task = tokio::spawn(async move {
        while let Some(msg) = framed_reader.next().await {
            let msg = msg?;
            if tx.send(msg).is_err() {
                break; // Channel closed
            }
        }
        Ok::<(), Box<dyn std::error::Error + Send + Sync>>(())
    });

    // Task 2: Write broadcast messages to this client's socket
    let write_task = tokio::spawn(async move {
        loop {
            let msg = match rx.recv().await {
                Ok(msg) => msg,
                Err(broadcast::error::RecvError::Lagged(_skipped)) => {
                    continue;
                }
                Err(broadcast::error::RecvError::Closed) => {
                     break;
                 }
            };
            if let Err(e) = framed_writer.send(msg).await {
                eprintln!("Error sending to client: {}", e);
                break;
            }
        }
        Ok::<(), Box<dyn std::error::Error + Send + Sync>>(())
    });

    // Wait for either task to complete (disconnect)
    tokio::select! {
        _ = read_task => {},
        _ = write_task => {},
    }
}
