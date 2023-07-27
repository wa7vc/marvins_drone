use std::thread;
use std::time::Duration;
use serde_json::json;

// for clap
use clap::Parser;
use linemux::MuxedLines;

// For phoenix_channels_client
use std::sync::Arc;
use tokio::sync::broadcast;
use phoenix_channels_client::{Event, EventPayload, Socket};
use url::Url;


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

    //#[arg(long, default_value = "wss://wa7vc.org/socket/websocket)]
    #[arg(long, default_value = "ws://127.0.0.1:11899/socket/websocket")]
    /// The URL for the Marvin to connect to
    marvin: String,

    //#[arg(long, default_value = "PRODUCTION_KEY_HERE")]
    #[arg(long, default_value = "devSecretKeyBaseMustBeAtLeast64BytesLong.ProductionUsesKeyFromAnsibleVault")]
    /// The shared secret key to use to connect to Marvin
    marvin_secret: String,
}

// Helper to get the default hostname into the Cli struct
fn system_hostname() -> String {
    match hostname::get() {
        Ok(names)   => names.to_string_lossy().to_string(),
        Err(e) => panic!("Could not determine system hostname. Either something is very wrong, or you should pass one manually with --hostname. Error was: {}", e.to_string()),
    }
}


fn ping_check_loop(ip: std::net::IpAddr, millis :u64) {
    loop {
        ping(ip);
        thread::sleep(Duration::from_millis(millis));
    }
}
fn ping(ip: std::net::IpAddr) {
    println!("I *should* ping {}... but I'm not going to.", ip);
}

#[tokio::main]
pub async fn main() -> std::io::Result<()> {
    let args = Cli::parse();

    let mut lines = MuxedLines::new()?;

    let marvin_url = Url::parse_with_params(
                        &args.marvin,
                        &[("shared_secret", args.marvin_secret)]
                     ).unwrap();
    let marvin_socket = Socket::spawn(marvin_url)
                            .await
                            .unwrap();
    marvin_socket
        .connect(Duration::from_secs(10))
        .await
        .unwrap_or_else(|_| panic!("Could not connect to Marvin at {}", args.marvin.as_str()));

    let cnc_channel = marvin_socket
                        .channel("drone:cnc", None)
                        .await
                        .unwrap();
    let mut cnc_event_receiver = cnc_channel.events();
    tokio::spawn(async move {
        loop {
            //let event = cnc_event_receiver.recv().await.unwrap();
            match cnc_event_receiver.recv().await {
                Ok(EventPayload { event, payload }) => match event {
                    Event::User(user_event_name) => println!("drone:cnc event {} sent with payload {:#?}", user_event_name, payload),
                    Event::Phoenix(phoenix_event) => println!("drone:cnc {}", phoenix_event),
                },
                Err(recv_error) => match recv_error {
                    broadcast::error::RecvError::Closed => break,
                    broadcast::error::RecvError::Lagged(lag) => {
                        eprintln!("drone:cnc events missed on channel {}", lag);
                    }
                }
            }
        }
    });
    cnc_channel
        .join(Duration::from_secs(15))
        .await
        .unwrap_or_else(|_| panic!("Could not join drone:cnc channel"));

    let pong = cnc_channel.call("ping", json!({ "hostname": args.hostname }), Duration::from_secs(5)).await.unwrap();
    println!("drone:cnc pong: {}", pong);

    /*** Phoenix Channel Setup ***/
    /*
    // Prepare configuration for the Phoenix Channel Client
    let mut config = Config::new(args.marvin.as_str()).unwrap();
    config.set("shared_secret", args.marvin_secret);

    // Create a client
    let mut client = Client::new(config)
                        .unwrap();

    // Connect the client
    client
        .connect()
        .await
        .unwrap_or_else(|_| panic!("Could not connect to Marvin at {}", args.marvin.as_str()));

    // Join a channel with no params and a timeout
    let cnc_channel = client
        .join("drone:cnc", Some(Duration::from_secs(15)))
        .await
        .unwrap_or_else(|_| panic!("Could not join drone:cnc channel"));

    let cnc_handler = cnc_channel
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
        .unwrap_or_else(|_| panic!("Got a channel error waiting for message on drone:cnc"));
    */ 
    // Send a message, waiting for a reply indefinitely
    //let result = channel.send("send_reply", json!({ "name": "foo", "message": "hi"})).await.unwrap();

    // Send a message, waiting for a reply with an optional timeout
    //let result = website_channel.send_with_timeout("ping", json!({ "text": "aaawwww jeez", "name": "foo", "message": "hello"}), Some(Duration::from_secs(5))).await.unwrap();

    // Send a message, not waiting for a reply
    //let result = website_channel.send_noreply("ping", json!({ "text": "awwwwwww jeez"})).await.unwrap();

    //println!("Result: {:?}", result);

    // Leave the channel
    //website_channel.leave().await;

    for ip in args.ping {
        let f_handle = thread::spawn(move || ping_check_loop(ip, 5000));
        f_handle.join().unwrap();
    }

    /*** Tail ***/
    for f in args.tail {
        lines.add_file(&f).await?;
        println!("Tailing file {}", &f.display());
    }

    // Note that this is an infinite loop, not a dispatch to an await loop. Which we should figure
    // out how to do.
    while let Ok(Some(line)) = lines.next_line().await {
        // Upload this line to Marvin
        println!("Sending line from {}:{} to Marvin: {}", args.hostname, line.source().display(), line.line());
        //let result = cnc_channel.send("drone_file_line", json!({ "hostname": "foo", "message": "hi"})).await.unwrap();
    }

    
    println!("Nothing to do apparently. Exiting.");
    Ok(())
}
