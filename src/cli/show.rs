use crate::config;
use clap::ArgMatches;
use clap::{arg, Command};
use colored::Colorize;
use log;
use std::env;
use syntect::easy::HighlightLines;
use syntect::highlighting::{Style, ThemeSet};
use syntect::parsing::SyntaxSet;
use syntect::util::{as_24_bit_terminal_escaped, LinesWithEndings};

const ABOUT: &str = r#"generate and show cloudformation template
"#;

pub fn command() -> Command {
    Command::new("show")
        .about(ABOUT.truecolor(125, 174, 189).to_string())
        .arg(arg!([stack]))
        .arg(arg!(-c --config <FILE> "path to config file"))
}

pub async fn handle(matches: &ArgMatches) -> Result<(), String> {
    log::debug!("initialising [show] command handler");
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

    let ps = SyntaxSet::load_defaults_newlines();
    let ts = ThemeSet::load_defaults();

    for stack in &conf.stacks {
        if stack.name.as_str() != stack_name.as_str() {
            continue;
        }

        let template = stack.generate_template()?;
        let syntax = ps.find_syntax_by_name("YAML").unwrap();

        let mut h = HighlightLines::new(syntax, &ts.themes["base16-ocean.dark"]);
        for line in LinesWithEndings::from(&template) {
            let ranges: Vec<(Style, &str)> = h
                .highlight_line(line, &ps)
                .map_err(|e| format!("error occured while formating response: {}", e))?;
            let escaped = as_24_bit_terminal_escaped(&ranges[..], false);
            print!("{}", escaped);
        }
        println!("");
    }

    Ok(())
}
