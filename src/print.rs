use std::fmt::Write;
use std::{io::IsTerminal, sync::LazyLock};

use anyhow::{Error, Result, bail};
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

fn quote(s: &str) -> String {
    Value::String(s.to_string()).to_string()
}

fn yaml_flow_string(s: &str) -> String {
    if s.starts_with(char::is_whitespace)
        || s.ends_with(char::is_whitespace)
        // Indicator characters
        || s.starts_with(|c: char| "-?:,[]{}#&*!|>'\"%@`".contains(c))
        // Characters that may start a number
        || s.starts_with(|c: char| "+-.".contains(c) || c.is_ascii_digit())
        // Strings that would be parsed as null, true, or false if unquoted.
        || ["null", "~", "true", "false"].contains(&s.to_lowercase().as_str())
        || s.contains(char::is_control)
        || s.contains(": ")
        || s.contains(" #")
    {
        quote(s)
    } else {
        s.to_string()
    }
}

fn yaml_block_string(depth: usize, s: &str) -> Result<String> {
    let mut res = String::from("|");
    if s.starts_with(char::is_whitespace) {
        write!(&mut res, "{TAB_WIDTH}")?;
    }
    for line in s.lines() {
        write!(&mut res, "\n{}{}", " ".repeat(depth * TAB_WIDTH), line)?;
    }
    Ok(res)
}

fn yaml_string(depth: usize, s: &str) -> Result<String> {
    if s.contains('\n') && !s.contains(|c: char| c.is_control() && c != '\n') {
        yaml_block_string(depth, s)
    } else {
        Ok(yaml_flow_string(s))
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
                    write_with_color!(w, KEY, "{}", yaml_flow_string(k))?;
                    write!(w, ":")?;
                    write_yaml(w, depth + 1, true, v)?;
                }
            }
        }
        Value::String(s) => {
            if obj_value {
                write!(w, " ")?;
            }
            let ys = yaml_string(depth, s)?;
            write_with_color!(w, STR, "{ys}")?;
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

fn toml_key(s: &str) -> String {
    // https://toml.io/en/v1.0.0#keys
    // A bare key must be non-empty.
    // Bare keys may only contain ASCII letters, ASCII digits, underscores, and dashes.
    if !s.is_empty()
        && s.chars()
            .all(|c| c.is_ascii_alphabetic() || c.is_ascii_digit() || c == '_' || c == '-')
    {
        s.to_string()
    } else {
        quote(s)
    }
}

fn toml_string(s: &str) -> String {
    if s.contains('\n') && !s.contains(|c: char| c.is_control() && c != '\n') && !s.contains("'''")
    {
        format!("'''\n{s}'''")
    } else {
        quote(s)
    }
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
                write_with_color!(w, KEY, " {}", toml_key(k))?;
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

fn write_toml(w: &mut impl WriteColor, context: &str, value: &Value) -> Result<()> {
    fn should_nest(value: &Value) -> bool {
        if let Value::Object(obj) = value {
            let values = obj.values().filter(|v| !v.is_null()).collect::<Vec<_>>();
            values.len() > 1 || (values.len() == 1 && should_nest(values[0]))
        } else if let Value::Array(arr) = value {
            arr.iter().all(Value::is_object)
        } else {
            false
        }
    }

    fn toml_key_value<'a>(k: &'a str, v: &'a Value) -> (String, &'a Value) {
        let k = toml_key(k);
        if let Value::Object(obj) = v {
            let obj = obj.iter().filter(|(_, v)| !v.is_null()).collect::<Vec<_>>();
            if obj.len() == 1 {
                let (inner_k, v) = toml_key_value(obj[0].0, obj[0].1);
                return (format!("{k}.{inner_k}"), v);
            }
        }
        (k, v)
    }

    match value {
        Value::Array(_) => write_toml_inline(w, value)?,
        Value::Object(obj) => {
            let obj = obj.iter().filter(|(_, v)| !v.is_null()).collect::<Vec<_>>();
            let flat = obj
                .iter()
                .filter(|(_, v)| !should_nest(v))
                .collect::<Vec<_>>();
            let nested = obj
                .iter()
                .filter(|(_, v)| should_nest(v))
                .collect::<Vec<_>>();

            for (i, &(k, v)) in flat.iter().enumerate() {
                let (k, v) = toml_key_value(k, v);
                write_with_color!(w, KEY, "{k}")?;
                write!(w, " = ")?;
                write_toml_inline(w, v)?;
                if i != flat.len() - 1 {
                    writeln!(w)?;
                }
            }

            for (i, &(k, v)) in nested.iter().enumerate() {
                let k = format!("{}{}", context, toml_key(k));
                if !flat.is_empty() || i > 0 {
                    write!(w, "\n\n")?;
                }
                match v {
                    Value::Object(obj) => {
                        if obj.iter().any(|(_, v)| !should_nest(v)) {
                            write_with_color!(w, HEADER, "[{k}]\n")?;
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
        Value::String(s) => write_with_color!(w, STR, "{}", toml_string(s))?,
        Value::Null => bail!("can't convert null to TOML"),
        _ => write!(w, "{value}")?,
    }
    Ok(())
}

pub fn json(w: &mut impl WriteColor, s: &str) -> Result<()> {
    write_json(w, 0, &s.parse()?)?;
    writeln!(w)?;
    Ok(())
}

pub fn yaml(w: &mut impl WriteColor, s: &str) -> Result<()> {
    write_yaml(w, 0, false, &s.parse()?)?;
    writeln!(w)?;
    Ok(())
}

pub fn toml(w: &mut impl WriteColor, s: &str) -> Result<()> {
    write_toml(w, "", &s.parse()?)?;
    writeln!(w)?;
    Ok(())
}

pub fn error(w: &mut impl WriteColor, err: &Error) -> Result<()> {
    write_with_color!(w, ERR, "error")?;
    writeln!(w, ": {err:#}")?;
    Ok(())
}

pub fn stdout() -> StandardStream {
    StandardStream::stdout(color_choice(&std::io::stdout()))
}

pub fn stderr() -> StandardStream {
    StandardStream::stderr(color_choice(&std::io::stderr()))
}
