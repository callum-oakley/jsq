#![warn(clippy::pedantic)]

mod color;
mod v8;

use std::{
    collections::HashMap,
    env,
    io::{self, IsTerminal, Read},
    process,
};

use anyhow::Result;
use clap::Parser;

pub const TAB_WIDTH: usize = 2;

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
    /// JSON.parse STDIN before passing it to the function.
    #[arg(short, long)]
    parse: bool,

    /// JSON.stringify the result before printing it to STDOUT.
    #[arg(short, long)]
    stringify: bool,

    /// The body of the JavaScript function to be evaluated.
    #[arg(default_value("$"))]
    body: String,
}

fn script(args: &Args) -> String {
    format!(
        "(() => {{ {} const res = (() => {})(); return {}; }})()",
        if args.parse { "$ = JSON.parse($);" } else { "" },
        args.body,
        if args.stringify {
            format!("JSON.stringify(res, null, {TAB_WIDTH})")
        } else {
            String::from("res")
        },
    )
}

fn vars() -> Result<HashMap<String, String>> {
    let mut vars = HashMap::new();

    let mut stdin = io::stdin();
    if !stdin.is_terminal() {
        let mut buf = String::new();
        stdin.read_to_string(&mut buf)?;
        vars.insert(String::from("$"), buf);
    }

    for (k, v) in env::vars() {
        vars.insert(format!("${k}"), v);
    }

    Ok(vars)
}

fn try_main() -> Result<()> {
    let args = Args::parse();

    let res = v8::eval(&script(&args), &vars()?)?;
    if args.stringify && io::stdout().is_terminal() {
        color::print_json(&res)?;
    } else if res.ends_with('\n') {
        print!("{res}");
    } else {
        println!("{res}");
    }

    Ok(())
}

fn main() {
    if let Err(err) = try_main() {
        color::print_error(&err).expect("printing error");
        process::exit(1);
    }
}
