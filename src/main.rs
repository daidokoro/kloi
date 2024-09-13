mod cli;
mod config;
mod logger;
mod stacks;

use clap::Command;
use cli::*;
use colored::Colorize;

const APP_NAME: &str = "kloi";
static VERSION: &str = env!("CARGO_PKG_VERSION");

fn about() -> String {
    let logo = r#"
     __  __     __         ______     __    
    /\ \/ /    /\ \       /\  __ \   /\ \   
    \ \  _"-.  \ \ \____  \ \ \/\ \  \ \ \  
     \ \_\ \_\  \ \_____\  \ \_____\  \ \_\ 
      \/_/\/_/   \/_____/   \/_____/   \/_/ 

    "#
    .truecolor(255, 146, 0);

    let desc =
        "A no-frills cli tool for managing aws cloudformation stacks".truecolor(125, 174, 189);

    format!("{}\n{}\n\n{}", logo, VERSION.green(), desc)
}

fn root_command() -> Command {
    Command::new(APP_NAME)
        .version(VERSION)
        .about(about())
        // add apply command
        .subcommand(cli::apply::command())
        // add delete command
        .subcommand(cli::delete::command())
        // add status command
        .subcommand(cli::status::command())
        // add show command
        .subcommand(cli::show::command())
        // add check command
        .subcommand(cli::check::command())
        // add completions command
        .subcommand(cli::completions::command())
}

#[tokio::main]
async fn main() -> Result<(), String> {
    logger::init();
    // define command
    let matches = root_command().get_matches();
    let r = match matches.subcommand() {
        Some(("apply", sub_matches)) => apply::handle(sub_matches).await,
        Some(("delete", sub_matches)) => delete::handle(sub_matches).await,
        Some(("status", sub_matches)) => status::handle(sub_matches).await,
        Some(("show", sub_matches)) => show::handle(sub_matches).await,
        Some(("check", sub_matches)) => check::handle(sub_matches).await,
        Some(("completions", sub_matches)) => completions::handle(sub_matches, root_command()),
        _ => root_command().print_help().map_err(|e| e.to_string()),
    };

    if let Err(e) = r {
        log::error!("{}", e.red());
        std::process::exit(1);
    }

    Ok(())
}
