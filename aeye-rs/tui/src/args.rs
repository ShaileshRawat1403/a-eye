use clap::Parser;
use std::path::PathBuf;

#[derive(Parser, Debug)]
#[command(name = "aeye-tui")]
#[command(about = "A-Eye Terminal UI", long_about = None)]
pub struct Args {
    #[arg(short, long, default_value = ".")]
    pub directory: PathBuf,

    #[arg(short, long)]
    pub model: Option<String>,

    #[arg(long)]
    pub nopretty: bool,
}
