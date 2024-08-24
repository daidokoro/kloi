use clap::{arg, value_parser, ArgAction, ArgMatches, Command};
use clap_complete::{generate, Generator, Shell};
use colored::Colorize;
use std::io;

const ABOUT: &str = "Generate shell completions for the current shell
";

pub fn command() -> Command {
    Command::new("completions")
        .about(ABOUT.truecolor(125, 174, 189).to_string())
        .arg(
            arg!([generator])
                .action(ArgAction::Set)
                .value_parser(value_parser!(Shell)),
        )
}

fn print_completions<G: Generator>(gen: G, cmd: &mut Command) {
    generate(gen, cmd, cmd.get_name().to_string(), &mut io::stdout());
}

pub fn handle(matches: &ArgMatches, mut cmd: Command) -> Result<(), String> {
    // let matches = cmd.clone().get_matches();

    if let Some(generator) = matches.get_one::<Shell>("generator").copied() {
        eprintln!("Generating completion file for {generator}...");
        print_completions(generator, &mut cmd);
    }

    Ok(())
}
