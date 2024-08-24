use aws_config::{self, BehaviorVersion};
use aws_types::region::Region;
use clap::ArgMatches;
use clap::{arg, Command};
use colored::Colorize;
use log;
use std::env;

use crate::config;
use crate::utils;
use crate::utils::stack_request_result_handle;
use utils::exec_jobs;

const ABOUT: &str = r#"deletes existing stacks
"#;

pub fn command() -> Command {
    Command::new("delete")
        .about(ABOUT.truecolor(125, 174, 189).to_string())
        .alias("d")
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

    let mut execution_stacks = match matches.get_one::<String>("stack") {
        Some(c) => {
            if let None = conf.stacks.iter().find(|s| &s.name == c) {
                Err(format!("stack [{}] not found", c))?;
            };
            vec![conf.stacks.iter().find(|s| &s.name == c).unwrap()]
        }

        // if no stack is specified, select all stacks
        None => conf.stacks.iter().collect(),
    };

    execution_stacks.sort_by(|a, b| {
        if a.is_dependency_of(b) {
            return std::cmp::Ordering::Less;
        }

        if b.is_dependency_of(a) {
            return std::cmp::Ordering::Greater;
        }

        std::cmp::Ordering::Equal
    });

    // reverse the order of execution_stacks
    execution_stacks.reverse();

    for stack in execution_stacks {
        // execute on_delete hook
        exec_jobs!(on_delete, &stack, stack.name.clone(), false);

        // // create client per stack
        let region = stack.region.clone().unwrap_or("eu-west-1".to_string());
        let sdk_config = aws_config::defaults(BehaviorVersion::latest())
            .region(Region::new(region))
            .load()
            .await;
        let client = aws_sdk_cloudformation::Client::new(&sdk_config);

        // delete stack
        let res = client
            .delete_stack()
            .stack_name(stack.name.clone())
            .send()
            .await;

        stack_request_result_handle!(res, stack.name, "delete stack");

        // wait for stack to be deleted
        utils::stackprogress(&client, &stack.name, stack.custom_resources.clone(), stack.region.clone().unwrap(), utils::WaitEvent::Delete).await?;

        // execute on_deleted hook
        exec_jobs!(on_delete, &stack, stack.name.clone(), true);
    }

    Ok(())
}
