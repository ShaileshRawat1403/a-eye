use aeye_tui::App;
use aeye_tui::Args;
use anyhow::Result;
use clap::Parser;

fn main() -> Result<()> {
    let args = Args::parse();
    let mut app = App::new(args)?;
    app.run()?;
    Ok(())
}
