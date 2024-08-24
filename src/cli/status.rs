use crate::config;
use crate::utils;
use aws_config::{self, BehaviorVersion};
use aws_types::region::Region;
use clap::ArgMatches;
use clap::{arg, Command};
use colored::Colorize;
use log;
use std::env;
use utils::exec_jobs;

const ABOUT: &str = r#"check status stacks based on config
"#;

pub fn command() -> Command {
    Command::new("status")
        .about(ABOUT.truecolor(125, 174, 189).to_string())
        .arg(arg!([stack]))
        .arg(arg!(-c --config <FILE> "path to config file"))
}

pub async fn handle(matches: &ArgMatches) -> Result<(), String> {
    let mut config_path = env::var("KLOI_CONFIG").ok();

    // if config is not set by env, check if it is set by cli
    if let None = config_path {
        log::debug!("config path is not set by env, KLOI_CONFIG, check CLI -c/--config");
        config_path = Some(matches
            .get_one::<String>("config")
            .ok_or_else(|| "config file required, please supply using -c/--config or set the KLOI_CONFIG env var".to_string())?.to_string());
    };

    // load config and create client
    // note: unwrap is fine here, since we've already checked if config is set above
    let conf = config::load_config_from_file(config_path.unwrap())?;
    for stack in &conf.stacks {
        // execute on_status hook
        exec_jobs!(on_status, &stack, stack.name.clone(), false);

        // // create client per stack
        let region = stack.region.clone().unwrap_or("eu-west-1".to_string());
        let sdk_config = aws_config::defaults(BehaviorVersion::latest())
            .region(Region::new(region))
            .load()
            .await;
        let client = aws_sdk_cloudformation::Client::new(&sdk_config);

        // get stack status
        let res = client
            .describe_stacks()
            .stack_name(stack.name.clone())
            .send()
            .await;

        match res {
            Ok(r) => {
                let s = r.stacks.unwrap().pop().unwrap();

                println!(
                    "[{}] {}",
                    s.stack_name.unwrap().cyan(),
                    s.stack_status.unwrap().as_str().to_lowercase().green()
                );
            }
            Err(e) => {
                let err = if let Some(message) = e.into_service_error().meta().message() {
                    format!(
                        "[{}] error occured while getting stack status: {}",
                        stack.name,
                        message.red()
                    )
                } else {
                    format!("[{}] {}", stack.name.cyan(), "unknown error".red())
                };

                if err.contains("does not exist") {
                    println!("[{}] {}", stack.name.cyan(), "does not exist".yellow());
                    return Ok(());
                }

                Err(err)?
            }
        }

        // execute post jobs
        exec_jobs!(on_status, &stack, stack.name.clone(), true);
    }

    Ok(())
}
