use tokio::net::TcpStream;
use tokio::io::{AsyncBufReadExt, BufReader};
use bytes::Bytes;

use async_bincode::tokio::{AsyncBincodeReader, AsyncBincodeWriter};
use futures::{SinkExt, StreamExt};

use my_redis::{ServerMessage, ClientMessage};

#[tokio::main]
async fn main() -> Result<(), Box<dyn std::error::Error>> {
    // Connect to the server
    let socket = TcpStream::connect("127.0.0.1:8080").await?;
    println!("Connected to server at 127.0.0.1:8080");

    let (reader, writer) = socket.into_split();

    // Wrap with async-bincode for automatic serialization
    use async_bincode::AsyncDestination;
    let mut framed_reader: AsyncBincodeReader<_, ServerMessage> = AsyncBincodeReader::from(reader);
    let mut framed_writer: AsyncBincodeWriter<_, ClientMessage, AsyncDestination> = AsyncBincodeWriter::from(writer).for_async();

    // Task 1: Read from server and print GameState updates
    let read_task = tokio::spawn(async move {
        while let Some(msg_result) = framed_reader.next().await {
            match msg_result {
                Ok(ServerMessage::GameState(state)) => {
                    println!("\n=== Game State Update ===");
                    if state.is_empty() {
                        println!("(empty)");
                    } else {
                        for (key, value) in &state {
                            println!("  {} = {}", key, String::from_utf8_lossy(value));
                        }
                    }
                    println!("=========================");
                    print!("\nEnter <key> <value>: ");
                    use std::io::Write;
                    std::io::stdout().flush().ok();
                }
                Err(e) => {
                    eprintln!("Error reading from server: {}", e);
                    break;
                }
            }
        }
    });

    // Task 2: Read from stdin and send to server
    let write_task = tokio::spawn(async move {
        let stdin = tokio::io::stdin();
        let mut reader = BufReader::new(stdin);
        let mut line = String::new();

        println!("Enter <key> <value> pairs to send to server:");
        print!("Enter <key> <value>: ");
        use std::io::Write;
        std::io::stdout().flush().ok();

        loop {
            line.clear();
            match reader.read_line(&mut line).await {
                Ok(0) => break, // EOF
                Ok(_) => {
                    let parts: Vec<&str> = line.trim().split_whitespace().collect();
                    if parts.len() >= 2 {
                        let key = parts[0].to_string();
                        let value = parts[1..].join(" ");

                        if let Err(e) = framed_writer.send(ClientMessage::Set(key.clone(), Bytes::from(value.clone()))).await {
                            eprintln!("Error sending to server: {}", e);
                            break;
                        }
                        println!("Sent: {} = {}", key, value);
                    } else if !line.trim().is_empty() {
                        println!("Invalid input. Please enter: <key> <value>");
                        print!("Enter <key> <value>: ");
                        std::io::stdout().flush().ok();
                    }
                }
                Err(e) => {
                    eprintln!("Error reading from stdin: {}", e);
                    break;
                }
            }
        }
        Ok::<(), Box<dyn std::error::Error + Send + Sync>>(())
    });

    // Wait for either task to complete
    tokio::select! {
        _ = read_task => println!("Server disconnected"),
        _ = write_task => println!("Input closed"),
    }

    Ok(())
}
