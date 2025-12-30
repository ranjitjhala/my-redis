use mini_redis::{Result, client};
use std::io::{self};

fn read_string(msg: &str) -> String {
    println!("{}", msg);
    let mut input = String::new();
    io::stdin().read_line(&mut input).unwrap();
    input.trim().to_string()
}

#[tokio::main]
async fn main() -> Result<()> {
    // Open a connection to the mini-redis address.
    let mut client = client::connect("127.0.0.1:6379").await?;

    // Prompt user for the key
    let key = read_string("Enter key:");

    let result = client.get(&key).await?;
    println!("Current value from the server => {result:?}");

    // Prompt user for the value
    let value = read_string("Enter value:");

    // update
    client.set(&key, value.into()).await?;

    // check updated value
    let result = client.get(&key).await?;
    println!("updated value at the server => {result:?}");

    Ok(())
}

use std::sync::Mutex;

struct CanIncrement {
    mutex: Mutex<i32>,
}
impl CanIncrement {
    // This function is not marked async.
    fn increment(&self) {
        let mut lock = self.mutex.lock().unwrap();
        *lock += 1;
    }
}

async fn increment_and_do_stuff(can_incr: &CanIncrement) {
    can_incr.increment();
    do_something_async(can_incr).await;
}

async fn do_something_async(can_incr: &CanIncrement) {
    // Simulate some asynchronous work.
    tokio::time::sleep(std::time::Duration::from_millis(100)).await;
}
