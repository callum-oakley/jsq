#![warn(clippy::pedantic)]

use std::{
    collections::HashMap,
    env,
    io::{self, IsTerminal, Read, Write},
    process,
    str::FromStr,
};

use anyhow::{bail, Context, Result};
use clap::Parser;
use serde_json::Value;
use termcolor::{Color, ColorChoice, ColorSpec, StandardStream, WriteColor};

const TAB_WIDTH: usize = 2;
const KEY_COLOR: Color = Color::Blue;
const STRING_COLOR: Color = Color::Green;

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

// Evaluate the given source, with the given vars in scope.
fn eval(vars: &HashMap<String, String>, source: &str) -> Result<String> {
    fn v8_string<'s>(
        scope: &mut v8::HandleScope<'s, ()>,
        s: &str,
    ) -> Result<v8::Local<'s, v8::String>> {
        v8::String::new(scope, s).context("constructing string")
    }

    let platform = v8::new_default_platform(0, false).make_shared();
    v8::V8::initialize_platform(platform);
    v8::V8::initialize();
    let mut isolate = v8::Isolate::new(v8::CreateParams::default());

    let mut scope = v8::HandleScope::new(&mut isolate);
    let object_template = v8::ObjectTemplate::new(&mut scope);
    for (k, v) in vars {
        object_template.set(
            v8_string(&mut scope, k)?.into(),
            v8_string(&mut scope, v)?.into(),
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

    let source = v8_string(&mut scope, source)?;
    let script = v8::Script::compile(&mut scope, source, None);
    if let Some(exception) = scope.exception() {
        bail!("{}", exception.to_rust_string_lossy(&mut scope))
    }

    let res = script.context("compiling script")?.run(&mut scope);
    if let Some(exception) = scope.exception() {
        bail!("{}", exception.to_rust_string_lossy(&mut scope))
    }

    Ok(res
        .context("running script")?
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
            format!("JSON.stringify(res, null, {TAB_WIDTH})")
        } else {
            String::from("res")
        },
    )
}

fn print_highlighted(s: &str) -> Result<()> {
    fn write_value(stdout: &mut StandardStream, depth: usize, value: &Value) -> Result<()> {
        match value {
            Value::Array(arr) => {
                if arr.is_empty() {
                    write!(stdout, "[]")?;
                } else {
                    writeln!(stdout, "[")?;
                    for (i, e) in arr.iter().enumerate() {
                        write!(stdout, "{}", " ".repeat((depth + 1) * TAB_WIDTH))?;
                        write_value(stdout, depth + 1, e)?;
                        if i == arr.len() - 1 {
                            writeln!(stdout)?;
                        } else {
                            writeln!(stdout, ",")?;
                        }
                    }
                    write!(stdout, "{}]", " ".repeat(depth * TAB_WIDTH))?;
                }
            }
            Value::Object(obj) => {
                if obj.is_empty() {
                    write!(stdout, "{{}}")?;
                } else {
                    writeln!(stdout, "{{")?;
                    for (i, (k, v)) in obj.iter().enumerate() {
                        stdout.set_color(ColorSpec::new().set_fg(Some(KEY_COLOR)))?;
                        write!(
                            stdout,
                            "{}{}",
                            " ".repeat((depth + 1) * TAB_WIDTH),
                            Value::String(k.clone()),
                        )?;
                        stdout.reset()?;
                        write!(stdout, ": ")?;
                        write_value(stdout, depth + 1, v)?;
                        if i == obj.len() - 1 {
                            writeln!(stdout)?;
                        } else {
                            writeln!(stdout, ",")?;
                        }
                    }
                    write!(stdout, "{}]", " ".repeat(depth * TAB_WIDTH))?;
                }
            }
            Value::String(_) => {
                stdout.set_color(ColorSpec::new().set_fg(Some(STRING_COLOR)))?;
                write!(stdout, "{value}")?;
                stdout.reset()?;
            }
            _ => write!(stdout, "{value}")?,
        }
        Ok(())
    }
    let mut stdout = StandardStream::stdout(ColorChoice::Auto);
    write_value(&mut stdout, 0, &Value::from_str(s)?)?;
    writeln!(&mut stdout)?;
    Ok(())
}

fn try_main() -> Result<()> {
    let args = Args::parse();

    let res = eval(&vars()?, &source(&args))?;
    if args.stringify && io::stdout().is_terminal() {
        print_highlighted(&res)?;
    } else if res.ends_with('\n') {
        print!("{res}");
    } else {
        println!("{res}");
    }

    Ok(())
}

fn main() {
    if let Err(err) = try_main() {
        let mut stderr = StandardStream::stderr(ColorChoice::Auto);
        stderr
            .set_color(ColorSpec::new().set_fg(Some(Color::Red)).set_bold(true))
            .expect("setting color");
        write!(&mut stderr, "error").expect("writing to stderr");
        stderr.reset().expect("resetting color");
        writeln!(&mut stderr, ": {err}").expect("writing to stderr");
        process::exit(1);
    }
}
