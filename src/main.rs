use clap::Parser;
use nexsh::NexSh;
use std::error::Error;
mod header;
pub mod prompt;
pub mod types;

#[derive(Parser, Debug)]
#[command(
    name = "nexsh",
    version = "0.2.0",
    about = "AI-powered smart shell using Google Gemini"
)]
struct Args {
    /// Execute single command and exit
    #[arg(short, long)]
    execute: Option<String>,
}

#[tokio::main]
async fn main() -> Result<(), Box<dyn Error>> {
    header::print_header();

    let args = parse_arguments();
    let mut shell = initialize_shell()?;

    if let Some(cmd) = args.execute {
        return handle_execute_command(cmd, &mut shell).await;
    }
    shell.run().await
}

fn parse_arguments() -> Args {
    Args::parse()
}

fn initialize_shell() -> Result<NexSh, Box<dyn Error>> {
    NexSh::new()
}

async fn handle_execute_command(cmd: String, shell: &mut NexSh) -> Result<(), Box<dyn Error>> {
    if cmd == "--help" || cmd == "-h" {
        shell.print_help()?;
        return Ok(());
    }
    shell.process_command(&cmd).await
}
