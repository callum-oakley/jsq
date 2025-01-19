use std::{
    io::{self, IsTerminal, Write},
    sync::LazyLock,
};

use anyhow::{Error, Result};
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

pub fn json(s: &str) -> Result<()> {
    fn write_value(w: &mut impl WriteColor, depth: usize, value: &serde_json::Value) -> Result<()> {
        match value {
            serde_json::Value::Array(arr) => {
                write!(w, "[")?;
                for (i, e) in arr.iter().enumerate() {
                    write!(w, "\n{}", " ".repeat((depth + 1) * TAB_WIDTH))?;
                    write_value(w, depth + 1, e)?;
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
                    write_value(w, depth + 1, v)?;
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

    let mut stdout = StandardStream::stdout(color_choice(&io::stdout()));
    let value = serde_json::from_str::<serde_json::Value>(s)?;
    write_value(&mut stdout, 0, &value)?;
    writeln!(&mut stdout)?;
    Ok(())
}

pub fn yaml(s: &str) -> Result<()> {
    fn write_value(
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
                        write_value(w, depth + 1, false, e)?;
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
                        write_value(w, depth + 1, true, v)?;
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

    let mut stdout = StandardStream::stdout(color_choice(&io::stdout()));
    let value = serde_json::from_str::<serde_yaml::Value>(s)?;
    write_value(&mut stdout, 0, false, &value)?;
    writeln!(&mut stdout)?;
    Ok(())
}

pub fn toml(s: &str) -> Result<()> {
    fn write_value(w: &mut impl WriteColor, context: &str, value: &toml::Value) -> Result<()> {
        match value {
            toml::Value::Array(arr) => {
                // TODO arrays of tables
                // TODO break long arrays over multiple lines
                write!(w, "[")?;
                for (i, e) in arr.iter().enumerate() {
                    write_value(w, context, e)?;
                    if i != arr.len() - 1 {
                        write!(w, ", ")?;
                    }
                }
                write!(w, "]")?;
            }
            toml::Value::Table(table) => {
                let mut flat = Vec::new();
                let mut nested = Vec::new();
                for (k, v) in table {
                    if matches!(v, toml::Value::Table(_)) {
                        nested.push((k, v));
                    } else {
                        flat.push((k, v));
                    }
                }

                for (i, &(k, v)) in flat.iter().enumerate() {
                    // TODO handle keys that need escaping
                    write_with_color!(w, KEY, "{k}")?;
                    write!(w, " = ")?;
                    write_value(w, context, v)?;
                    if i != flat.len() - 1 {
                        writeln!(w)?;
                    }
                }

                for (i, &(k, v)) in nested.iter().enumerate() {
                    if !flat.is_empty() || i > 0 {
                        write!(w, "\n\n")?;
                    }
                    // TODO handle keys that need escaping
                    write_with_color!(w, HEADER, "[{context}{k}]")?;
                    let toml::Value::Table(table) = v else {
                        unreachable!("nested only contains tables by construction");
                    };
                    if !table.is_empty() {
                        writeln!(w)?;
                    }
                    // TODO handle keys that need escaping
                    write_value(w, &format!("{context}{k}."), v)?;
                }
            }
            toml::Value::String(_) => write_with_color!(w, STR, "{value}")?,
            _ => write!(w, "{value}")?,
        }
        Ok(())
    }

    let mut stdout = StandardStream::stdout(color_choice(&io::stdout()));
    let value = serde_json::from_str::<toml::Value>(s)?;
    write_value(&mut stdout, "", &value)?;
    writeln!(&mut stdout)?;
    Ok(())
}

pub fn error(err: &Error) -> Result<()> {
    let mut stderr = StandardStream::stderr(color_choice(&io::stderr()));
    write_with_color!(&mut stderr, ERR, "error")?;
    writeln!(&mut stderr, ": {err:#}")?;
    Ok(())
}
