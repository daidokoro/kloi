use crate::config;
use aws_config::{self, BehaviorVersion};
use aws_sdk_cloudformation::error::SdkError;
use aws_types::region::Region;
use clap::{arg, ArgMatches, Command};
use colored::Colorize;
use log;
use std::env;
use crate::utils;
use tempdir::TempDir;
use std::fs::File;
use std::io::Write;

const ABOUT: &str = r#"validate cloudformation template
This command uses cfn-lint if present on the host, else it will use the AWS Cloudformation validation API
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
    let stack_name: String = match matches.get_one::<String>("stack") {
        Some(c) => {
            if let None = conf.stacks.iter().find(|s| &s.name == c) {
                Err(format!("stack [{}] not found", c))?;
            };
            c.to_string()
        }
        None => {
            let opts = conf
                .stacks
                .iter()
                .map(|s| s.name.clone())
                .collect::<Vec<String>>();
            utils::singleselect(opts, "select stack")
        },
    };

    // can be unwrapped because we already checked that the stack exists
    let stack = conf.stacks.iter().find(|s| &s.name == &stack_name).unwrap();
    let template = stack.generate_template()?;

    if let Ok(res) = call_cfn_lint(template.clone()) {
        if res.contains("no issues found") {
            println!("{}", res);
            return Ok(());
        }
        // return Ok result as error to retain err code
        // on bad cfn-lint output
        return Err(format!("\n---\n{}", res));
    }

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
                "{}\n---\n{} no issues found",
                template.truecolor(96, 96, 96),
                "✔︎".green()
            );

            Ok(())
        }
        Err(e) => match e {
            SdkError::ServiceError(sdk_err) => {
                println!("{}", template.truecolor(96, 96, 96));
                let err = format!(
                    "error occured while validating template: {}",
                    sdk_err.into_err().meta().message().unwrap_or("unknown error")
                );

                Err(err)
            }

            _ => Err(e.to_string()),
        },
    }
}


// run cfn-lint on the template string as a subprocess
fn call_cfn_lint(template: String) -> Result<String, String> {
    
    let (cfn_lint_path, _) = utils::sh!("which cfn-lint");
    if cfn_lint_path.contains("not found") {
        return Err("cfn-lint not found".to_string());
    };

    log::debug!("cfn-lint path detected: {}", cfn_lint_path.trim());

    // create temporary file with template string
    let tmp_dir = TempDir::new("kloi_check")
        .map_err(|e| format!("failed to create temporary directory: {}", e.to_string()))?;
    let tmp_template_path_buf = tmp_dir.path().join("template.yaml");
    let path = tmp_template_path_buf.to_string_lossy().to_string();
    let mut tmp_config_file = File::create(&tmp_template_path_buf)
        .map_err(|e| format!("failed to create temporary file: {}", e.to_string()))?;
    write!(tmp_config_file, "{}", template).unwrap();

    let cmd = format!("{} {}", cfn_lint_path.trim(), path.trim());
    log::debug!("running cfn-lint: {}", cmd);

    let (stdout, stderr) = utils::sh!(cmd);
    if stderr != "" {
        return Err(stderr);
    }

    if stdout == "" {
        return Ok(format!("{} no issues found", "✔︎".green()));
    }

    Ok(stdout.replace(&path, "#"))
}   