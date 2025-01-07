use std::{
    io::{self, IsTerminal, Write},
    str::FromStr,
    sync::LazyLock,
};

use anyhow::{Error, Result};
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
static ERR: LazyLock<ColorSpec> = LazyLock::new(|| bold(Color::Red));

macro_rules! write_with_color {
    ($dst:expr, $color:expr, $($arg:tt)*) => {
        $dst.set_color(&$color)
            .and_then(|_| write!($dst, $($arg)*))
            .and_then(|_| $dst.reset())
    };
}

pub fn json(s: &str) -> Result<()> {
    fn write_value(w: &mut impl WriteColor, depth: usize, value: &Value) -> Result<()> {
        match value {
            Value::Array(arr) => {
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
            Value::Object(obj) => {
                write!(w, "{{")?;
                for (i, (k, v)) in obj.iter().enumerate() {
                    write!(w, "\n{}", " ".repeat((depth + 1) * TAB_WIDTH))?;
                    write_with_color!(w, KEY, "{}", Value::String(k.clone()))?;
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
            Value::String(_) => write_with_color!(w, STR, "{value}")?,
            _ => write!(w, "{value}")?,
        }
        Ok(())
    }
    let mut stdout = StandardStream::stdout(if io::stdout().is_terminal() {
        ColorChoice::Auto
    } else {
        ColorChoice::Never
    });
    write_value(&mut stdout, 0, &Value::from_str(s)?)?;
    writeln!(&mut stdout)?;
    Ok(())
}

pub fn error(err: &Error) -> Result<()> {
    let mut stderr = StandardStream::stderr(if io::stderr().is_terminal() {
        ColorChoice::Auto
    } else {
        ColorChoice::Never
    });
    write_with_color!(&mut stderr, ERR, "error")?;
    writeln!(&mut stderr, ": {err:#}")?;
    Ok(())
}
