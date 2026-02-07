#![warn(clippy::pedantic)]

mod deno;
mod parse;
mod print;

use std::io::{IsTerminal, Read};

use anyhow::{Context, Result};
use clap::Parser;
use deno::{Options, Print};

/// Evaluate some JavaScript and print the result.
#[derive(Parser)]
#[command(
    version,
    arg_required_else_help(true),
    after_help([
        "Input is available in SCRIPT as $.",
        "Environment variables are available in SCRIPT prefixed by $.",
    ].join(" "))
)]
#[expect(clippy::struct_excessive_bools)]
struct Args {
    /// Parse input as JSON.
    #[arg(short('j'), long, conflicts_with_all(["yaml_in", "toml_in", "json5_in"]))]
    json_in: bool,

    /// Parse input as YAML.
    #[arg(short('y'), long, conflicts_with_all(["json_in", "toml_in", "json5_in"]))]
    yaml_in: bool,

    /// Parse input as TOML.
    #[arg(short('t'), long, conflicts_with_all(["json_in", "yaml_in", "json5_in"]))]
    toml_in: bool,

    /// Parse input as JSON5.
    #[arg(short('5'), long, conflicts_with_all(["json_in", "yaml_in", "toml_in"]))]
    json5_in: bool,

    /// Print result as JSON.
    #[arg(short('J'), long, conflicts_with_all(["yaml_out", "toml_out", "json5_out", "no_out"]))]
    json_out: bool,

    /// Print result as YAML.
    #[arg(short('Y'), long, conflicts_with_all(["json_out", "toml_out", "json5_out", "no_out"]))]
    yaml_out: bool,

    /// Print result as TOML.
    #[arg(short('T'), long, conflicts_with_all(["json_out", "yaml_out", "json5_out", "no_out"]))]
    toml_out: bool,

    /// Print result as JSON5.
    #[arg(short('%'), long, conflicts_with_all(["json_out", "yaml_out", "toml_out", "no_out"]))]
    json5_out: bool,

    /// Don't print result.
    #[arg(short('N'), long, conflicts_with_all(["json_out", "yaml_out", "toml_out", "json5_out"]))]
    no_out: bool,

    /// The JavaScript to be evaluated.
    #[arg(default_value("$"), conflicts_with("file"))]
    script: String,

    /// Read SCRIPT from FILE.
    #[arg(short('f'), long, conflicts_with("script"))]
    file: Option<String>,
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
    } else if args.json5_in {
        input = parse::json5(&input)?;
    }

    let script = if let Some(f) = args.file {
        std::fs::read_to_string(f)?
    } else {
        args.script
    };

    let print = if args.no_out {
        Print::None
    } else if args.json_out || args.yaml_out || args.toml_out || args.json5_out {
        Print::Object
    } else {
        Print::String
    };

    let output = deno::eval(Options {
        input: &input,
        env: std::env::vars(),
        script: &script,
        parse: args.json_in || args.yaml_in || args.toml_in || args.json5_in,
        print,
    })?;

    if let Some(value) = output {
        if args.json_out {
            print::json(&mut print::stdout(), &value).context("printing JSON")?;
        } else if args.yaml_out {
            print::yaml(&mut print::stdout(), &value).context("printing YAML")?;
        } else if args.toml_out {
            print::toml(&mut print::stdout(), &value).context("printing TOML")?;
        } else if args.json5_out {
            print::json5(&mut print::stdout(), &value).context("printing JSON5")?;
        }
    }

    Ok(())
}

fn main() {
    if let Err(err) = try_main() {
        print::error(&mut print::stderr(), &err).expect("printing error");
        std::process::exit(1);
    }
}
