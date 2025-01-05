#![warn(clippy::pedantic)]

use std::{
    collections::HashMap,
    env,
    io::{self, IsTerminal, Read, Write},
    process,
};

use anyhow::{bail, Result};
use clap::Parser;
use termcolor::{Color, ColorChoice, ColorSpec, StandardStream, WriteColor};

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

    /// The BODY of the JavaScript function to be evaluated.
    #[arg(default_value("$"))]
    body: String,
}

// Evaluate the given source, with the given vars in scope.
fn eval(vars: &HashMap<String, String>, source: &str) -> Result<String> {
    fn v8_string<'s>(scope: &mut v8::HandleScope<'s, ()>, s: &str) -> v8::Local<'s, v8::String> {
        v8::String::new(scope, s).expect("constructing string")
    }

    let platform = v8::new_default_platform(0, false).make_shared();
    v8::V8::initialize_platform(platform);
    v8::V8::initialize();
    let mut isolate = v8::Isolate::new(v8::CreateParams::default());

    let mut scope = v8::HandleScope::new(&mut isolate);
    let object_template = v8::ObjectTemplate::new(&mut scope);
    for (k, v) in vars {
        object_template.set(
            v8_string(&mut scope, k).into(),
            v8_string(&mut scope, v).into(),
        );
    }
    let context = v8::Context::new(
        &mut scope,
        v8::ContextOptions {
            global_template: Some(object_template),
            global_object: None,
            microtask_queue: None,
        },
    );
    let mut scope = v8::ContextScope::new(&mut scope, context);
    let mut scope = v8::TryCatch::new(&mut scope);

    let source = v8_string(&mut scope, source);
    let script = v8::Script::compile(&mut scope, source, None);
    if let Some(exception) = scope.exception() {
        bail!("{}", exception.to_rust_string_lossy(&mut scope))
    }

    let res = script.expect("compiling script").run(&mut scope);
    if let Some(exception) = scope.exception() {
        bail!("{}", exception.to_rust_string_lossy(&mut scope))
    }

    Ok(res
        .expect("running script")
        .to_rust_string_lossy(&mut scope))
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

fn source(args: &Args) -> String {
    format!(
        "(() => {{ {} const res = (() => {})(); return {}; }})()",
        if args.parse { "$ = JSON.parse($);" } else { "" },
        args.body,
        if args.stringify {
            "JSON.stringify(res, null, 2)"
        } else {
            "res"
        },
    )
}

fn main() -> Result<()> {
    let args = Args::parse();

    match eval(&vars()?, &source(&args)) {
        Ok(res) => {
            if res.ends_with('\n') {
                print!("{res}");
            } else {
                println!("{res}");
            }
        }
        Err(err) => {
            let mut stderr = StandardStream::stderr(ColorChoice::Auto);
            stderr.set_color(ColorSpec::new().set_fg(Some(Color::Red)).set_bold(true))?;
            write!(&mut stderr, "error")?;
            stderr.reset()?;
            writeln!(&mut stderr, ": {err}")?;
            process::exit(1);
        }
    }

    Ok(())
}
