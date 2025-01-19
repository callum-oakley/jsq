use std::{
    io::{self, IsTerminal, Write},
    sync::LazyLock,
};

use anyhow::{Error, Result};
use regex::Regex;
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

fn write_json(w: &mut impl WriteColor, depth: usize, value: &serde_json::Value) -> Result<()> {
    match value {
        serde_json::Value::Array(arr) => {
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
        serde_json::Value::Object(obj) => {
            write!(w, "{{")?;
            for (i, (k, v)) in obj.iter().enumerate() {
                write!(w, "\n{}", " ".repeat((depth + 1) * TAB_WIDTH))?;
                write_with_color!(w, KEY, "{}", serde_json::Value::String(k.clone()))?;
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
        serde_json::Value::String(_) => write_with_color!(w, STR, "{value}")?,
        _ => write!(w, "{value}")?,
    }
    Ok(())
}

fn write_yaml(
    w: &mut impl WriteColor,
    depth: usize,
    map_value: bool,
    value: &serde_yaml::Value,
) -> Result<()> {
    match value {
        serde_yaml::Value::Sequence(seq) => {
            if seq.is_empty() {
                if map_value {
                    write!(w, " ")?;
                }
                write!(w, "[]")?;
            } else {
                for (i, e) in seq.iter().enumerate() {
                    if i > 0 || map_value {
                        write!(w, "\n{}", " ".repeat(depth * TAB_WIDTH))?;
                    }
                    write!(w, "- ")?;
                    write_yaml(w, depth + 1, false, e)?;
                }
            }
        }
        serde_yaml::Value::Mapping(map) => {
            if map.is_empty() {
                if map_value {
                    write!(w, " ")?;
                }
                write!(w, "{{}}")?;
            } else {
                for (i, (k, v)) in map.iter().enumerate() {
                    if i > 0 || map_value {
                        write!(w, "\n{}", " ".repeat(depth * TAB_WIDTH))?;
                    }
                    let k = serde_yaml::to_string(k)?;
                    write_with_color!(w, KEY, "{}", k.trim())?;
                    write!(w, ":")?;
                    write_yaml(w, depth + 1, true, v)?;
                }
            }
        }
        serde_yaml::Value::String(_) => {
            if map_value {
                write!(w, " ")?;
            }
            let value = serde_yaml::to_string(value)?;
            // Replace serde_yaml's default 2 space indentation for block scalars.
            let value = value.replace("\n  ", &format!("\n{}", " ".repeat(depth * TAB_WIDTH)));
            write_with_color!(w, STR, "{}", value.trim())?;
        }
        _ => {
            if map_value {
                write!(w, " ")?;
            }
            write!(w, "{}", serde_yaml::to_string(value)?.trim())?;
        }
    }
    Ok(())
}

fn toml_key(key: &str) -> String {
    static RE: LazyLock<Regex> = LazyLock::new(|| Regex::new(r"^[A-Za-z0-9_-]+$").unwrap());
    if RE.is_match(key) {
        key.to_string()
    } else {
        // https://toml.io/en/v1.0.0#string
        // Any Unicode character may be used except those that must be escaped: quotation mark,
        // backslash, and the control characters other than tab (U+0000 to U+0008, U+000A to U+001F,
        // U+007F).
        format!(
            "\"{}\"",
            key.replace('"', "\\\"")
                .replace('\\', "\\\\")
                .replace('\u{0000}', "\\u0000")
                .replace('\u{0001}', "\\u0001")
                .replace('\u{0002}', "\\u0002")
                .replace('\u{0003}', "\\u0003")
                .replace('\u{0004}', "\\u0004")
                .replace('\u{0005}', "\\u0005")
                .replace('\u{0006}', "\\u0006")
                .replace('\u{0007}', "\\u0007")
                .replace('\u{0008}', "\\b")
                .replace('\u{0009}', "\\t")
                .replace('\u{000A}', "\\n")
                .replace('\u{000B}', "\\u000B")
                .replace('\u{000C}', "\\f")
                .replace('\u{000D}', "\\r")
                .replace('\u{000E}', "\\u000E")
                .replace('\u{000F}', "\\u000F")
                .replace('\u{0010}', "\\u0010")
                .replace('\u{0011}', "\\u0011")
                .replace('\u{0012}', "\\u0012")
                .replace('\u{0013}', "\\u0013")
                .replace('\u{0014}', "\\u0014")
                .replace('\u{0015}', "\\u0015")
                .replace('\u{0016}', "\\u0016")
                .replace('\u{0017}', "\\u0017")
                .replace('\u{0018}', "\\u0018")
                .replace('\u{0019}', "\\u0019")
                .replace('\u{001A}', "\\u001A")
                .replace('\u{001B}', "\\u001B")
                .replace('\u{001C}', "\\u001C")
                .replace('\u{001D}', "\\u001D")
                .replace('\u{001E}', "\\u001E")
                .replace('\u{001F}', "\\u001F")
                .replace('\u{007F}', "\\u007F")
        )
    }
}

fn write_toml_inline(w: &mut impl WriteColor, value: &toml::Value) -> Result<()> {
    match value {
        toml::Value::Array(arr) => {
            write!(w, "[")?;
            for (i, e) in arr.iter().enumerate() {
                write_toml_inline(w, e)?;
                if i != arr.len() - 1 {
                    write!(w, ", ")?;
                }
            }
            write!(w, "]")?;
        }
        toml::Value::Table(table) => {
            write!(w, "{{")?;
            for (i, (k, v)) in table.iter().enumerate() {
                write_with_color!(w, KEY, " {}", toml_key(k))?;
                write!(w, " = ")?;
                write_toml_inline(w, v)?;
                if i == table.len() - 1 {
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

fn write_toml(w: &mut impl WriteColor, context: &str, value: &toml::Value) -> Result<()> {
    fn is_table(value: &toml::Value) -> bool {
        matches!(value, toml::Value::Table(_))
    }

    fn is_table_array(value: &toml::Value) -> bool {
        if let toml::Value::Array(arr) = value {
            arr.iter().all(is_table)
        } else {
            false
        }
    }

    match value {
        toml::Value::Array(_) => write_toml_inline(w, value)?,
        toml::Value::Table(table) => {
            let mut flat = Vec::new();
            let mut nested = Vec::new();
            for (k, v) in table {
                if is_table(v) || is_table_array(v) {
                    nested.push((k, v));
                } else {
                    flat.push((k, v));
                }
            }

            for (i, &(k, v)) in flat.iter().enumerate() {
                write_with_color!(w, KEY, "{}", toml_key(k))?;
                write!(w, " = ")?;
                write_toml(w, context, v)?;
                if i != flat.len() - 1 {
                    writeln!(w)?;
                }
            }

            for (i, &(k, v)) in nested.iter().enumerate() {
                let k = format!("{}{}", context, toml_key(k));
                if !flat.is_empty() || i > 0 {
                    write!(w, "\n\n")?;
                }
                // TODO omit unnecessary headers
                match v {
                    toml::Value::Table(table) => {
                        write_with_color!(w, HEADER, "[{k}]")?;
                        if !table.is_empty() {
                            writeln!(w)?;
                        }
                        write_toml(w, &format!("{k}."), v)?;
                    }
                    toml::Value::Array(arr) => {
                        for (i, e) in arr.iter().enumerate() {
                            if i > 0 {
                                write!(w, "\n\n")?;
                            }
                            let toml::Value::Table(table) = e else {
                                unreachable!("arr only contains tables by construction");
                            };
                            write_with_color!(w, HEADER, "[[{k}]]")?;
                            if !table.is_empty() {
                                writeln!(w)?;
                            }
                            write_toml(w, &format!("{k}."), e)?;
                        }
                    }
                    _ => unreachable!("nested contains tables and arrays by construction"),
                }
            }
        }
        toml::Value::String(_) => write_with_color!(w, STR, "{value}")?,
        _ => write!(w, "{value}")?,
    }
    Ok(())
}

pub fn json(s: &str) -> Result<()> {
    let mut stdout = StandardStream::stdout(color_choice(&io::stdout()));
    let value = serde_json::from_str::<serde_json::Value>(s)?;
    write_json(&mut stdout, 0, &value)?;
    writeln!(&mut stdout)?;
    Ok(())
}

pub fn yaml(s: &str) -> Result<()> {
    let mut stdout = StandardStream::stdout(color_choice(&io::stdout()));
    let value = serde_json::from_str::<serde_yaml::Value>(s)?;
    write_yaml(&mut stdout, 0, false, &value)?;
    writeln!(&mut stdout)?;
    Ok(())
}

pub fn toml(s: &str) -> Result<()> {
    let mut stdout = StandardStream::stdout(color_choice(&io::stdout()));
    let value = serde_json::from_str::<toml::Value>(s)?;
    write_toml(&mut stdout, "", &value)?;
    writeln!(&mut stdout)?;
    Ok(())
}

pub fn error(err: &Error) -> Result<()> {
    let mut stderr = StandardStream::stderr(color_choice(&io::stderr()));
    write_with_color!(&mut stderr, ERR, "error")?;
    writeln!(&mut stderr, ": {err:#}")?;
    Ok(())
}
