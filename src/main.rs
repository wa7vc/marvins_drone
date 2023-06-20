use clap::Parser;
use linemux::MuxedLines;

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
