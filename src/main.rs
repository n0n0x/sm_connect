mod aws;
use aws::InstanceInfo;
mod app;
mod ui;
use app::App;
use std::process::Command;
mod components;
mod history;
use history::{History, HistoryEntry};
mod screens;
use anyhow::Result;
use signal_hook::{consts::signal::*, iterator::Signals};
use clap::Parser;
use aws_config::Region;
use crossterm::{
    execute,
    terminal::{Clear, ClearType, disable_raw_mode},
    cursor::MoveTo,
};
use std::io::{self, Write};

/// AWS Systems Manager Session Manager connection tool
#[derive(Parser, Debug)]
#[command(author, version, about, long_about = None)]
struct Args {
    /// AWS region (e.g., us-east-1, us-west-2)
    #[arg(short, long)]
    region: Option<String>,

    /// AWS EC2 instance ID (e.g., i-ad53d5e3831ea)
    #[arg(short, long)]
    instance: Option<String>,
}

#[tokio::main]
async fn main() -> Result<()> {
    let args = Args::parse();

    // If both region and instance are provided, connect directly
    if let (Some(region_str), Some(instance_id)) = (args.region, args.instance) {
        let region = Region::new(region_str);
        let instance = InstanceInfo::new(region, instance_id);
        return connect(instance);
    }

    // Otherwise, run the interactive UI
    let mut app = App::new()?;
    let result = app.run().await;
    
    // Explicitly drop the app to ensure terminal is restored before connecting
    drop(app);
    
    match result {
        Ok(instance) => connect(instance),
        Err(e) => match e.downcast_ref::<app::RuntimeError>() {
            Some(app::RuntimeError::UserExit) => Ok(()),
            _ => {
                eprintln!("Error: {:?}", e);
                Ok(())
            }
        },
    }
}

fn connect(instance: InstanceInfo) -> Result<()> {
    // Save to history before connecting
    let entry = HistoryEntry::new(instance.get_instance_id());
    History::save(entry)?;
    
    // Make sure raw mode is disabled
    disable_raw_mode()?;
    
    // Clear the screen and move cursor to top-left before connecting
    let mut stdout = io::stdout();
    execute!(stdout, Clear(ClearType::All), MoveTo(0, 0))?;
    stdout.flush()?;
    
    // Run the AWS command
    let mut child = Command::new("aws")
        .args([
            "--region",
            instance.get_region().as_ref(),
            "ssm",
            "start-session",
            "--target",
            instance.get_instance_id(),
        ])
        .spawn()?;

    // Catch SIGINT, SIGSTP signal and do nothing
    // So that actually ctrl+c / ctrl+z works on the aws ssm session instead of killing / stopping us
    let _signals = Signals::new([SIGINT, SIGTSTP])?;

    child.wait()?;
    Ok(())
}
