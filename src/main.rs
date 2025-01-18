#![warn(clippy::pedantic)]

mod print;
mod v8;

use std::{
    env,
    io::{self, IsTerminal, Read},
    process,
};

use anyhow::Result;
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
struct Args {
    /// Parse STDIN as JSON before passing it to the function.
    #[arg(short, long)]
    parse: bool,

    /// Parse STDIN as TOML before passing it to the function.
    #[arg(short, long)]
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
        parse: args.parse || args.toml,
        stringify: args.stringify,
        body: &args.body,
        stdin: String::new(),
        env: env::vars(),
    };

    let mut stdin = io::stdin();
    if !stdin.is_terminal() {
        stdin.read_to_string(&mut options.stdin)?;
    }

    if args.toml {
        options.stdin = options
            .stdin
            .parse::<toml::Value>()?
            .try_into::<serde_json::Value>()?
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
