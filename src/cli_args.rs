use std::sync::LazyLock;

use clap::Parser;

/// TUI application to view project status.
#[derive(Parser, Debug)]
#[command(version, about, long_about = None)]
pub struct Args {
    /// Path to database
    #[arg(short, long, default_value = "./data.db")]
    pub db_path: String,
}

pub static ARGS: LazyLock<Args> = LazyLock::new(|| Args::parse());
