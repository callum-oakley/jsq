#![warn(clippy::pedantic)]

mod boa;
mod parse;
mod print;

use std::io::{IsTerminal, Read};

use anyhow::{anyhow, Context, Result};
use boa::Options;
use clap::Parser;

/// Evaluate a JavaScript function and print the result.
#[derive(Parser)]
#[command(
    version,
    arg_required_else_help(true),
    after_help([
        "Input is avaialable in BODY as $.",
        "Environment variables are available in BODY prefixed by $.",
    ].join(" "))
)]
#[expect(clippy::struct_excessive_bools)]
struct Args {
    /// Parse input as JSON.
    #[arg(short('j'), long, conflicts_with_all(["yaml_in", "toml_in"]))]
    json_in: bool,

    /// Parse input as YAML.
    #[arg(short('y'), long, conflicts_with_all(["json_in", "toml_in"]))]
    yaml_in: bool,

    /// Parse input as TOML.
    #[arg(short('t'), long, conflicts_with_all(["json_in", "yaml_in"]))]
    toml_in: bool,

    /// Print output as JSON.
    #[arg(short('J'), long, conflicts_with_all(["yaml_out", "toml_out"]))]
    json_out: bool,

    /// Print output as YAML.
    #[arg(short('Y'), long, conflicts_with_all(["json_out", "toml_out"]))]
    yaml_out: bool,

    /// Print output as TOML.
    #[arg(short('T'), long, conflicts_with_all(["json_out", "yaml_out"]))]
    toml_out: bool,

    /// The body of the JavaScript function to be evaluated.
    #[arg(default_value("$"))]
    body: String,
}

fn try_main() -> Result<()> {
    let args = Args::parse();

    let mut input = String::new();

    let mut stdin = std::io::stdin();
    if !stdin.is_terminal() {
        stdin.read_to_string(&mut input)?;
    }

    if args.json_in {
        input = parse::json(&input)?;
    } else if args.yaml_in {
        input = parse::yaml(&input)?;
    } else if args.toml_in {
        input = parse::toml(&input)?;
    }

    let res = boa::eval(Options {
        input: &input,
        env: std::env::vars(),
        body: &args.body,
        parse: args.json_in || args.yaml_in || args.toml_in,
        stringify: args.json_out || args.yaml_out || args.toml_out,
    })
    .map_err(|err| anyhow!("{err}"))?;

    // undefined is a valid output of JSON.stringify
    if args.json_out && res != "undefined" {
        print::json(&mut print::stdout(), &res).context("printing JSON")?;
    } else if args.yaml_out && res != "undefined" {
        print::yaml(&mut print::stdout(), &res).context("printing YAML")?;
    } else if args.toml_out && res != "undefined" {
        print::toml(&mut print::stdout(), &res).context("printing TOML")?;
    } else if res.ends_with('\n') {
        print!("{res}");
    } else {
        println!("{res}");
    }

    Ok(())
}

fn main() {
    if let Err(err) = try_main() {
        print::error(&mut print::stderr(), &err).expect("printing error");
        std::process::exit(1);
    }
}
