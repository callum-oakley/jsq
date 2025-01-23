use std::{
    io::{self, IsTerminal, Write},
    sync::LazyLock,
};

use anyhow::{bail, Error, Result};
use regex::Regex;
use serde_json::Value;
use termcolor::{Color, ColorChoice, ColorSpec, StandardStream, WriteColor};

const TAB_WIDTH: usize = 2;

fn normal(color: Color) -> ColorSpec {
    let mut spec = ColorSpec::new();
    spec.set_fg(Some(color));
    spec
}

fn bold(color: Color) -> ColorSpec {
    let mut spec = normal(color);
    spec.set_bold(true);
    spec
}

static KEY: LazyLock<ColorSpec> = LazyLock::new(|| normal(Color::Blue));
static STR: LazyLock<ColorSpec> = LazyLock::new(|| normal(Color::Green));
static HEADER: LazyLock<ColorSpec> = LazyLock::new(|| bold(Color::Blue));
static ERR: LazyLock<ColorSpec> = LazyLock::new(|| bold(Color::Red));

macro_rules! write_with_color {
    ($dst:expr, $color:expr, $($arg:tt)*) => {
        $dst.set_color(&$color)
            .and_then(|_| write!($dst, $($arg)*))
            .and_then(|_| $dst.reset())
    };
}

fn color_choice(t: &impl IsTerminal) -> ColorChoice {
    if t.is_terminal() {
        ColorChoice::Auto
    } else {
        ColorChoice::Never
    }
}

fn write_json(w: &mut impl WriteColor, depth: usize, value: &Value) -> Result<()> {
    match value {
        Value::Array(arr) => {
            write!(w, "[")?;
            for (i, e) in arr.iter().enumerate() {
                write!(w, "\n{}", " ".repeat((depth + 1) * TAB_WIDTH))?;
                write_json(w, depth + 1, e)?;
                if i == arr.len() - 1 {
                    write!(w, "\n{}", " ".repeat(depth * TAB_WIDTH))?;
                } else {
                    write!(w, ",")?;
                }
            }
            write!(w, "]")?;
        }
        Value::Object(obj) => {
            write!(w, "{{")?;
            for (i, (k, v)) in obj.iter().enumerate() {
                write!(w, "\n{}", " ".repeat((depth + 1) * TAB_WIDTH))?;
                write_with_color!(w, KEY, "{}", Value::String(k.clone()))?;
                write!(w, ": ")?;
                write_json(w, depth + 1, v)?;
                if i == obj.len() - 1 {
                    write!(w, "\n{}", " ".repeat(depth * TAB_WIDTH))?;
                } else {
                    write!(w, ",")?;
                }
            }
            write!(w, "}}")?;
        }
        Value::String(_) => write_with_color!(w, STR, "{value}")?,
        _ => write!(w, "{value}")?,
    }
    Ok(())
}

// Conservative quoting to try and cover strings that need quoting in both YAML and TOML. No harm in
// quoting more than necessary.
// TODO split out a separate, less conservative function for YAML.
fn quote_weird(s: &str) -> String {
    static RE: LazyLock<Regex> =
        LazyLock::new(|| Regex::new(r"^[A-Za-z][A-Za-z0-9_\-]*$").unwrap());
    if RE.is_match(s) {
        s.to_string()
    } else {
        Value::String(s.to_string()).to_string()
    }
}

fn write_yaml(w: &mut impl WriteColor, depth: usize, obj_value: bool, value: &Value) -> Result<()> {
    match value {
        Value::Array(arr) => {
            if arr.is_empty() {
                if obj_value {
                    write!(w, " ")?;
                }
                write!(w, "[]")?;
            } else {
                for (i, e) in arr.iter().enumerate() {
                    if i > 0 || obj_value {
                        write!(w, "\n{}", " ".repeat(depth * TAB_WIDTH))?;
                    }
                    write!(w, "- ")?;
                    write_yaml(w, depth + 1, false, e)?;
                }
            }
        }
        Value::Object(obj) => {
            if obj.is_empty() {
                if obj_value {
                    write!(w, " ")?;
                }
                write!(w, "{{}}")?;
            } else {
                for (i, (k, v)) in obj.iter().enumerate() {
                    if i > 0 || obj_value {
                        write!(w, "\n{}", " ".repeat(depth * TAB_WIDTH))?;
                    }
                    write_with_color!(w, KEY, "{}", quote_weird(k))?;
                    write!(w, ":")?;
                    write_yaml(w, depth + 1, true, v)?;
                }
            }
        }
        Value::String(s) => {
            if obj_value {
                write!(w, " ")?;
            }
            // TODO block scalars
            write_with_color!(w, STR, "{}", quote_weird(s))?;
        }
        _ => {
            if obj_value {
                write!(w, " ")?;
            }
            write!(w, "{value}")?;
        }
    }
    Ok(())
}

fn write_toml_inline(w: &mut impl WriteColor, value: &Value) -> Result<()> {
    match value {
        Value::Array(arr) => {
            let arr = arr.iter().filter(|v| !v.is_null()).collect::<Vec<_>>();
            write!(w, "[")?;
            for (i, e) in arr.iter().enumerate() {
                write_toml_inline(w, e)?;
                if i != arr.len() - 1 {
                    write!(w, ", ")?;
                }
            }
            write!(w, "]")?;
        }
        Value::Object(obj) => {
            let obj = obj.iter().filter(|(_, v)| !v.is_null()).collect::<Vec<_>>();
            write!(w, "{{")?;
            for (i, (k, v)) in obj.iter().enumerate() {
                write_with_color!(w, KEY, " {}", quote_weird(k))?;
                write!(w, " = ")?;
                write_toml_inline(w, v)?;
                if i == obj.len() - 1 {
                    write!(w, " ")?;
                } else {
                    write!(w, ",")?;
                }
            }
            write!(w, "}}")?;
        }
        _ => write_toml(w, "", value)?,
    }
    Ok(())
}

// TODO omit unnecessary headers
// TODO write objects with a single key using a dotted key rather than a new header
fn write_toml(w: &mut impl WriteColor, context: &str, value: &Value) -> Result<()> {
    fn is_object_array(value: &Value) -> bool {
        if let Value::Array(arr) = value {
            arr.iter().all(Value::is_object)
        } else {
            false
        }
    }

    match value {
        Value::Array(_) => write_toml_inline(w, value)?,
        Value::Object(obj) => {
            let obj = obj.iter().filter(|(_, v)| !v.is_null()).collect::<Vec<_>>();
            let mut flat = Vec::new();
            let mut nested = Vec::new();
            for (k, v) in obj {
                if v.is_object() || is_object_array(v) {
                    nested.push((k, v));
                } else {
                    flat.push((k, v));
                }
            }

            for (i, &(k, v)) in flat.iter().enumerate() {
                write_with_color!(w, KEY, "{}", quote_weird(k))?;
                write!(w, " = ")?;
                write_toml(w, context, v)?;
                if i != flat.len() - 1 {
                    writeln!(w)?;
                }
            }

            for (i, &(k, v)) in nested.iter().enumerate() {
                let k = format!("{}{}", context, quote_weird(k));
                if !flat.is_empty() || i > 0 {
                    write!(w, "\n\n")?;
                }
                match v {
                    Value::Object(obj) => {
                        write_with_color!(w, HEADER, "[{k}]")?;
                        if !obj.is_empty() {
                            writeln!(w)?;
                        }
                        write_toml(w, &format!("{k}."), v)?;
                    }
                    Value::Array(arr) => {
                        for (i, e) in arr.iter().enumerate() {
                            if i > 0 {
                                write!(w, "\n\n")?;
                            }
                            let Value::Object(obj) = e else {
                                unreachable!("arr only contains objects by construction");
                            };
                            write_with_color!(w, HEADER, "[[{k}]]")?;
                            if !obj.is_empty() {
                                writeln!(w)?;
                            }
                            write_toml(w, &format!("{k}."), e)?;
                        }
                    }
                    _ => unreachable!("nested contains objects and arrays by construction"),
                }
            }
        }
        Value::String(_) => write_with_color!(w, STR, "{value}")?,
        Value::Null => bail!("can't convert null to TOML"),
        _ => write!(w, "{value}")?,
    }
    Ok(())
}

pub fn json(s: &str) -> Result<()> {
    let mut stdout = StandardStream::stdout(color_choice(&io::stdout()));
    write_json(&mut stdout, 0, &s.parse()?)?;
    writeln!(&mut stdout)?;
    Ok(())
}

pub fn yaml(s: &str) -> Result<()> {
    let mut stdout = StandardStream::stdout(color_choice(&io::stdout()));
    write_yaml(&mut stdout, 0, false, &s.parse()?)?;
    writeln!(&mut stdout)?;
    Ok(())
}

pub fn toml(s: &str) -> Result<()> {
    let mut stdout = StandardStream::stdout(color_choice(&io::stdout()));
    write_toml(&mut stdout, "", &s.parse()?)?;
    writeln!(&mut stdout)?;
    Ok(())
}

pub fn error(err: &Error) -> Result<()> {
    let mut stderr = StandardStream::stderr(color_choice(&io::stderr()));
    write_with_color!(&mut stderr, ERR, "error")?;
    writeln!(&mut stderr, ": {err:#}")?;
    Ok(())
}
