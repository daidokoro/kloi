use crate::config;
use aws_config::{self, BehaviorVersion};
use aws_sdk_cloudformation::error::SdkError;
use aws_types::region::Region;
use clap::{arg, ArgMatches, Command};
use colored::Colorize;
use log;
use std::env;

const ABOUT: &str = r#"validate cloudformation template
"#;

pub fn command() -> Command {
    Command::new("check")
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
    let stack_name = match matches.get_one::<String>("stack") {
        Some(c) => {
            if let None = conf.stacks.iter().find(|s| &s.name == c) {
                Err(format!("stack [{}] not found", c))?;
            };
            c
        }
        None => return Err("stack name required, please supply using [stack]".to_string()),
    };

    // can be unwrapped because we already checked that the stack exists
    let stack = conf.stacks.iter().find(|s| &s.name == stack_name).unwrap();
    let template = stack.generate_template()?;

    // create client
    let region = stack.region.clone().unwrap_or("eu-west-1".to_string());
    let sdk_config = aws_config::defaults(BehaviorVersion::latest())
        .region(Region::new(region))
        .load()
        .await;
    let client = aws_sdk_cloudformation::Client::new(&sdk_config);

    // validate template
    let res = client
        .validate_template()
        .template_body(&template)
        .send()
        .await;

    match res {
        Ok(_) => {
            println!(
                "{}\n---\n{}",
                template.truecolor(96, 96, 96),
                "template is valid".green()
            );

            Ok(())
        }
        Err(e) => match e {
            SdkError::ServiceError(sdk_err) => {
                println!("{}", template.truecolor(96, 96, 96));
                let err = format!(
                    "error occured while validating template: {}",
                    sdk_err.into_err().to_string()
                );

                Err(err)
            }

            _ => Err(e.to_string()),
        },
    }
}
