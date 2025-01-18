#![warn(clippy::pedantic)]

mod print;
mod v8;

use std::{
    env,
    io::{self, IsTerminal, Read},
    process,
};

use anyhow::{Context, Result};
use clap::Parser;

/// Evaluate a JavaScript function and print the result.
#[derive(Parser)]
#[command(
    version,
    arg_required_else_help(true),
    after_help([
        "STDIN is avaialable in BODY as $.",
        "Environment variables are available in BODY prefixed by $.",
    ].join(" "))
)]
#[expect(clippy::struct_excessive_bools)]
struct Args {
    /// Parse STDIN as JSON before passing it to the function.
    #[arg(short, long)]
    parse: bool,

    /// Parse STDIN as YAML before passing it to the function.
    #[arg(short, long, conflicts_with("toml"))]
    yaml: bool,

    /// Parse STDIN as TOML before passing it to the function.
    #[arg(short, long, conflicts_with("yaml"))]
    toml: bool,

    /// JSON.stringify the result before printing it to STDOUT.
    #[arg(short, long)]
    stringify: bool,

    /// The body of the JavaScript function to be evaluated.
    #[arg(default_value("$"))]
    body: String,
}

fn try_main() -> Result<()> {
    let args = Args::parse();

    let mut options = v8::Options {
        parse: args.parse || args.yaml || args.toml,
        stringify: args.stringify,
        body: &args.body,
        stdin: String::new(),
        env: env::vars(),
    };

    let mut stdin = io::stdin();
    if !stdin.is_terminal() {
        stdin.read_to_string(&mut options.stdin)?;
    }

    if args.yaml {
        // TODO serde_yaml is deprecated.
        // Keeping an eye on https://github.com/saphyr-rs/saphyr/issues/1.
        options.stdin = serde_yaml::from_str::<serde_json::Value>(&options.stdin)
            .context("parsing YAML")?
            .to_string();
    }

    if args.toml {
        options.stdin = toml::from_str::<serde_json::Value>(&options.stdin)
            .context("parsing TOML")?
            .to_string();
    }

    let res = v8::eval(options)?;
    // JSON.stringify can return undefined, which isn't JSON, so don't try to highlight it.
    if args.stringify && res != "undefined" {
        print::json(&res)?;
    } else if res.ends_with('\n') {
        print!("{res}");
    } else {
        println!("{res}");
    }

    Ok(())
}

fn main() {
    if let Err(err) = try_main() {
        print::error(&err).expect("printing error");
        process::exit(1);
    }
}
