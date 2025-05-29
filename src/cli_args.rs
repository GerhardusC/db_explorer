use std::sync::LazyLock;

use clap::Parser;

/// TUI application to view project status.
#[derive(Parser, Debug)]
#[command(version, about, long_about = None)]
pub struct Args {
    /// Path to database
    #[arg(short, long, default_value = "./data.db")]
    pub db_path: String,
    /// Initial host to connect to via mqtt
    #[arg(short, long, default_value = "oldlaptop.local")]
    pub broker_ip: String,
    /// Initial topic to subscribe to via mqtt
    #[arg(short, long, default_value = "/#")]
    pub topic: String,
}

pub static ARGS: LazyLock<Args> = LazyLock::new(|| Args::parse());
