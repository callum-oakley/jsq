#![warn(clippy::pedantic)]

mod deno;
mod parse;
mod print;

use std::{
    fs::File,
    io::{IsTerminal, Read},
    mem,
};

use anyhow::{Context, Result};
use clap::Parser;
use deno::{Options, Print};
use serde_json::Value;
use termcolor::{NoColor, WriteColor};

/// Read data from STDIN, manipulate it with some JavaScript, write the result to STDOUT.
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
    #[arg(short('j'), long, conflicts_with_all(["yaml_in", "toml_in", "json5_in", "csv_in"]))]
    json_in: bool,

    /// Parse input as YAML.
    #[arg(short('y'), long, conflicts_with_all(["json_in", "toml_in", "json5_in", "csv_in"]))]
    yaml_in: bool,

    /// Parse input as TOML.
    #[arg(short('t'), long, conflicts_with_all(["json_in", "yaml_in", "json5_in", "csv_in"]))]
    toml_in: bool,

    /// Parse input as JSON5.
    #[arg(short('5'), long, conflicts_with_all(["json_in", "yaml_in", "toml_in", "csv_in"]))]
    json5_in: bool,

    /// Parse input as CSV.
    #[arg(short('c'), long, conflicts_with_all(["json_in", "yaml_in", "toml_in", "json5_in"]))]
    csv_in: bool,

    /// Print result as JSON.
    #[arg(short('J'), long, conflicts_with_all(["yaml_out", "toml_out", "json5_out", "csv_out", "no_out"]))]
    json_out: bool,

    /// Print result as YAML.
    #[arg(short('Y'), long, conflicts_with_all(["json_out", "toml_out", "json5_out", "csv_out", "no_out"]))]
    yaml_out: bool,

    /// Print result as TOML.
    #[arg(short('T'), long, conflicts_with_all(["json_out", "yaml_out", "json5_out", "csv_out", "no_out"]))]
    toml_out: bool,

    /// Print result as JSON5.
    #[arg(short('%'), long, conflicts_with_all(["json_out", "yaml_out", "toml_out", "csv_out", "no_out"]))]
    json5_out: bool,

    /// Print result as CSV.
    #[arg(short('C'), long, conflicts_with_all(["json_out", "yaml_out", "toml_out", "json5_out", "no_out"]))]
    csv_out: bool,

    /// Don't print result.
    #[arg(short('N'), long, conflicts_with_all(["json_out", "yaml_out", "toml_out", "json5_out", "csv_out"]))]
    no_out: bool,

    /// Print object keys in sorted order.
    #[arg(short('s'), long)]
    sort: bool,

    /// Read SCRIPT from a file.
    #[arg(short('f'), long)]
    file: bool,

    /// Edit INPUT file in-place.
    #[arg(short('i'), long, requires("input"))]
    in_place: bool,

    /// The JavaScript to be evaluated.
    #[arg(default_value("$"))]
    script: String,

    /// File to read input from instead of STDIN.
    input: Option<String>,
}

fn try_main() -> Result<()> {
    let mut args = Args::parse();

    let script = if args.file {
        std::fs::read_to_string(mem::take(&mut args.script))?
    } else {
        mem::take(&mut args.script)
    };

    let mut input = String::new();

    if let Some(f) = &args.input {
        input = std::fs::read_to_string(f)?;
    } else {
        let mut stdin = std::io::stdin();
        if !stdin.is_terminal() {
            stdin.read_to_string(&mut input)?;
        }
    }

    if args.json_in {
        input = parse::json(&input)?;
    } else if args.yaml_in {
        input = parse::yaml(&input)?;
    } else if args.toml_in {
        input = parse::toml(&input)?;
    } else if args.json5_in {
        input = parse::json5(&input)?;
    } else if args.csv_in {
        input = parse::csv(&input)?;
    }

    let print = if args.no_out {
        Print::None
    } else if args.json_out || args.yaml_out || args.toml_out || args.json5_out || args.csv_out {
        Print::Object
    } else {
        Print::String
    };

    let output = deno::eval(Options {
        input: &input,
        env: std::env::vars(),
        script: &script,
        parse: args.json_in || args.yaml_in || args.toml_in || args.json5_in || args.csv_in,
        print,
    })?;

    if let Some(value) = output {
        fn print(args: &Args, value: &Value, w: &mut impl WriteColor) -> Result<()> {
            if args.json_out {
                print::json(w, value).context("printing JSON")?;
            } else if args.yaml_out {
                print::yaml(w, value).context("printing YAML")?;
            } else if args.toml_out {
                print::toml(w, value).context("printing TOML")?;
            } else if args.json5_out {
                print::json5(w, value).context("printing JSON5")?;
            } else if args.csv_out {
                print::csv(w, value).context("printing CSV")?;
            }
            Ok(())
        }

        let value = if args.sort {
            print::sort(&value)
        } else {
            value
        };

        if args.in_place {
            print(
                &args,
                &value,
                &mut NoColor::new(File::create(
                    args.input.as_ref().expect("--in-place requires --input"),
                )?),
            )?;
        } else {
            print(&args, &value, &mut print::stdout())?;
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
