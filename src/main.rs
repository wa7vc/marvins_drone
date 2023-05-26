use clap::Parser;
use linemux::MuxedLines;

use std::time::Duration;
//use serde_json::json;
use phoenix_channels_client::Payload;
use phoenix_channels_client::{Config, Client};
//use std::pin::Pin;
use std::sync::Arc;

#[derive(Parser)]
#[command(name = "One of Marvin's mindless drones")]
#[command(author = "WA7VC Radio Club <info@wa7vc.org>")]
#[command(version = "0.1.0")]
#[command(about = "Uploads data to Marvin, so that he can know all things and therefore be even more depressed about everything. Also retrieves data from Marvin to provide to local systems.", long_about = None)]
struct Cli {
    #[arg(long, default_value_t = system_hostname())]
    /// The hostname of this device, so Marvin knows where the data came from.
    hostname: String,

    #[arg(short, long)]
    /// Tail the given file(s), uploading each line to Marvin as it comes in
    tail: Vec<std::path::PathBuf>,

    #[arg(short, long)]
    /// Ping the given IP addr(s) every 5 seconds
    ping: Vec<std::net::IpAddr>,

    #[arg(long, default_value = "https://wa7vc.org/marvin")]
    /// The URL for the Marvin to connect to
    marvin: String,
}

// Helper to get the default hostname into the Cli struct
fn system_hostname() -> String {
    match hostname::get() {
        Ok(names)   => names.to_string_lossy().to_string(),
        Err(e) => panic!("Could not determine system hostname. Either something is very wrong, or you should pass one manually with --hostname. Error was: {}", e.to_string()),
    }
}


#[tokio::main]
pub async fn main() -> std::io::Result<()> {
    let args = Cli::parse();

    let mut lines = MuxedLines::new()?;


    /*** Phoenix Channel Setup ***/
    // Prepare configuration for the client
    let mut config = Config::new("ws://10.110.42.31:11899/socket/websocket?token=undefined").unwrap();
    //let mut config = Config::new("wss://wa7vc.org/socket/websocket?token=undefined").unwrap();
    config.set("shared_secret", "devSecretKeyBaseMustBeAtLeast64BytesLong.ProducionUsesKeyFromAnsibleVault");

    // Create a client
    let mut client = Client::new(config).unwrap();

    // Connect the client
    client.connect().await.unwrap();

    // Join a channel with no params and a timeout
    let website_channel = client.join("website:pingmsg", Some(Duration::from_secs(15))).await.unwrap();

    // Register an event handler, save the ref returned and use `off` to unsubscribe
    let website_handler = website_channel
        .on(
            "message",
            Box::new(
                move |channel: Arc<phoenix_channels_client::Channel>, payload: &Payload| {
                    println!(
                        "channel received {} from topic '{}'",
                        payload,
                        channel.topic()
                    );
                },
            ),
        )
        .await
        .unwrap();
 
    // Send a message, waiting for a reply indefinitely
    //let result = channel.send("send_reply", json!({ "name": "foo", "message": "hi"})).await.unwrap();

    // Send a message, waiting for a reply with an optional timeout
    //let result = website_channel.send_with_timeout("ping", json!({ "text": "aaawwww jeez", "name": "foo", "message": "hello"}), Some(Duration::from_secs(5))).await.unwrap();

    // Send a message, not waiting for a reply
    //let result = website_channel.send_noreply("ping", json!({ "text": "awwwwwww jeez"})).await.unwrap();

    //println!("Result: {:?}", result);

    // Leave the channel
    //website_channel.leave().await;


    /*** Tail ***/
    for f in args.tail {
        lines.add_file(&f).await?;
        println!("Tailing file {}", &f.display());
    }

    while let Ok(Some(line)) = lines.next_line().await {
        // Upload this line to Marvin
        println!("Sending line from {}:{} to Marvin: {}", args.hostname, line.source().display(), line.line());
    }


    Ok(())
}
