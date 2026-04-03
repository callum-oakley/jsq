use std::{io::IsTerminal, sync::LazyLock};

use anyhow::{Context, Error, Result, anyhow, bail};
use indexmap::IndexSet;
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

fn with_color<W, F, E>(w: &mut W, color: &ColorSpec, mut f: F) -> Result<()>
where
    W: WriteColor,
    F: FnMut(&mut W) -> std::result::Result<(), E>,
    E: Into<Error>,
{
    w.set_color(color)?;
    f(w).map_err(|err| anyhow!(err))?;
    w.reset()?;
    Ok(())
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
                write!(w, "\n{:indent$}", "", indent = (depth + 1) * TAB_WIDTH)?;
                write_json(w, depth + 1, e)?;
                if i == arr.len() - 1 {
                    write!(w, "\n{:indent$}", "", indent = depth * TAB_WIDTH)?;
                } else {
                    write!(w, ",")?;
                }
            }
            write!(w, "]")?;
        }
        Value::Object(obj) => {
            write!(w, "{{")?;
            for (i, (k, v)) in obj.iter().enumerate() {
                write!(w, "\n{:indent$}", "", indent = (depth + 1) * TAB_WIDTH)?;
                with_color(w, &KEY, |w| serde_json::to_writer(w, k))?;
                write!(w, ": ")?;
                write_json(w, depth + 1, v)?;
                if i == obj.len() - 1 {
                    write!(w, "\n{:indent$}", "", indent = depth * TAB_WIDTH)?;
                } else {
                    write!(w, ",")?;
                }
            }
            write!(w, "}}")?;
        }
        Value::String(_) => with_color(w, &STR, |w| serde_json::to_writer(w, value))?,
        _ => serde_json::to_writer(w, value)?,
    }
    Ok(())
}

fn write_yaml_flow_string(w: &mut impl WriteColor, s: &str) -> Result<()> {
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
        serde_json::to_writer(w, s)?;
    } else {
        write!(w, "{s}")?;
    }
    Ok(())
}

fn write_yaml_block_string(w: &mut impl WriteColor, depth: usize, s: &str) -> Result<()> {
    write!(w, "|")?;
    if s.starts_with(char::is_whitespace) {
        write!(w, "{TAB_WIDTH}")?;
    }
    for line in s.lines() {
        write!(w, "\n{:indent$}{}", "", line, indent = depth * TAB_WIDTH,)?;
    }
    Ok(())
}

fn write_yaml_string(w: &mut impl WriteColor, depth: usize, s: &str) -> Result<()> {
    if s.contains('\n') && !s.contains(|c: char| c.is_control() && c != '\n') {
        write_yaml_block_string(w, depth, s)
    } else {
        write_yaml_flow_string(w, s)
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
                        write!(w, "\n{:indent$}", "", indent = depth * TAB_WIDTH)?;
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
                        write!(w, "\n{:indent$}", "", indent = depth * TAB_WIDTH)?;
                    }
                    with_color(w, &KEY, |w| write_yaml_flow_string(w, k))?;
                    write!(w, ":")?;
                    write_yaml(w, depth + 1, true, v)?;
                }
            }
        }
        Value::String(s) => {
            if obj_value {
                write!(w, " ")?;
            }
            with_color(w, &STR, |w| write_yaml_string(w, depth, s))?;
        }
        _ => {
            if obj_value {
                write!(w, " ")?;
            }
            serde_json::to_writer(w, value)?;
        }
    }
    Ok(())
}

fn write_toml_key(w: &mut impl WriteColor, key: &[&str]) -> Result<()> {
    for (i, s) in key.iter().enumerate() {
        if i > 0 {
            write!(w, ".")?;
        }

        // https://toml.io/en/v1.0.0#keys
        // A bare key must be non-empty.
        // Bare keys may only contain ASCII letters, ASCII digits, underscores, and dashes.
        if !s.is_empty()
            && s.chars()
                .all(|c| c.is_ascii_alphabetic() || c.is_ascii_digit() || c == '_' || c == '-')
        {
            write!(w, "{s}")?;
        } else {
            serde_json::to_writer(&mut *w, s)?;
        }
    }
    Ok(())
}

fn write_toml_string(w: &mut impl WriteColor, s: &str) -> Result<()> {
    if s.contains('\n') && !s.contains(|c: char| c.is_control() && c != '\n') && !s.contains("'''")
    {
        write!(w, "'''\n{s}'''")?;
    } else {
        serde_json::to_writer(w, s)?;
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
                write!(w, " ")?;
                with_color(w, &KEY, |w| write_toml_key(w, &[k.as_ref()]))?;
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
        _ => write_toml(w, &mut Vec::new(), value)?,
    }
    Ok(())
}

fn write_toml<'a>(
    w: &mut impl WriteColor,
    context: &mut Vec<&'a str>,
    value: &'a Value,
) -> Result<()> {
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

    fn write_toml_key_value<'a>(
        w: &mut impl WriteColor,
        k: &'a str,
        mut v: &'a Value,
    ) -> Result<()> {
        let mut key = vec![k];
        while let Value::Object(obj) = v {
            let obj = obj.iter().filter(|(_, v)| !v.is_null()).collect::<Vec<_>>();
            if obj.len() == 1 {
                key.push(obj[0].0);
                v = obj[0].1;
            }
        }
        with_color(w, &KEY, |w| write_toml_key(w, &key))?;
        write!(w, " = ")?;
        write_toml_inline(w, v)
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
                write_toml_key_value(w, k, v)?;
                if i != flat.len() - 1 {
                    writeln!(w)?;
                }
            }

            for (i, &(k, v)) in nested.iter().enumerate() {
                context.push(k);
                if !flat.is_empty() || i > 0 {
                    write!(w, "\n\n")?;
                }
                match v {
                    Value::Object(obj) => {
                        if obj.iter().any(|(_, v)| !should_nest(v)) {
                            with_color(w, &HEADER, |w| -> Result<()> {
                                write!(w, "[")?;
                                write_toml_key(w, context)?;
                                writeln!(w, "]")?;
                                Ok(())
                            })?;
                        }
                        write_toml(w, context, v)?;
                    }
                    Value::Array(arr) => {
                        for (i, e) in arr.iter().enumerate() {
                            if i > 0 {
                                write!(w, "\n\n")?;
                            }
                            let Value::Object(obj) = e else {
                                unreachable!("arr only contains objects by construction");
                            };
                            with_color(w, &HEADER, |w| -> Result<()> {
                                write!(w, "[[")?;
                                write_toml_key(w, context)?;
                                writeln!(w, "]]")?;
                                Ok(())
                            })?;
                            if !obj.is_empty() {
                                writeln!(w)?;
                            }
                            write_toml(w, context, e)?;
                        }
                    }
                    _ => unreachable!("nested contains objects and arrays by construction"),
                }
                context.pop();
            }
        }
        Value::String(s) => with_color(w, &STR, |w| write_toml_string(w, s))?,
        Value::Null => bail!("can't convert null to TOML"),
        _ => serde_json::to_writer(w, value)?,
    }
    Ok(())
}

fn write_json5_key(w: &mut impl WriteColor, k: &str) -> Result<()> {
    let mut chars = k.chars();
    if let Some(first) = chars.next()
        && json5::char::is_json5_identifier_start(first)
        && chars.all(json5::char::is_json5_identifier)
    {
        with_color(w, &KEY, |w| write!(w, "{k}"))?;
    } else {
        with_color(w, &KEY, |w| json5::to_writer(w, &k))?;
    }
    Ok(())
}

fn write_json5(w: &mut impl WriteColor, depth: usize, value: &Value) -> Result<()> {
    match value {
        Value::Array(arr) => {
            write!(w, "[")?;
            for (i, e) in arr.iter().enumerate() {
                write!(w, "\n{:indent$}", "", indent = (depth + 1) * TAB_WIDTH)?;
                write_json5(w, depth + 1, e)?;
                if i == arr.len() - 1 {
                    write!(w, ",\n{:indent$}", "", indent = depth * TAB_WIDTH)?;
                } else {
                    write!(w, ",")?;
                }
            }
            write!(w, "]")?;
        }
        Value::Object(obj) => {
            write!(w, "{{")?;
            for (i, (k, v)) in obj.iter().enumerate() {
                write!(w, "\n{:indent$}", "", indent = (depth + 1) * TAB_WIDTH)?;
                write_json5_key(w, k)?;
                write!(w, ": ")?;
                write_json5(w, depth + 1, v)?;
                if i == obj.len() - 1 {
                    write!(w, ",\n{:indent$}", "", indent = depth * TAB_WIDTH)?;
                } else {
                    write!(w, ",")?;
                }
            }
            write!(w, "}}")?;
        }
        Value::String(_) => {
            with_color(w, &STR, |w| json5::to_writer(w, value))?;
        }
        _ => json5::to_writer(w, value)?,
    }
    Ok(())
}

pub fn json(w: &mut impl WriteColor, value: &Value) -> Result<()> {
    write_json(w, 0, value)?;
    writeln!(w)?;
    Ok(())
}

pub fn yaml(w: &mut impl WriteColor, value: &Value) -> Result<()> {
    write_yaml(w, 0, false, value)?;
    writeln!(w)?;
    Ok(())
}

pub fn toml(w: &mut impl WriteColor, value: &Value) -> Result<()> {
    write_toml(w, &mut Vec::new(), value)?;
    writeln!(w)?;
    Ok(())
}

pub fn json5(w: &mut impl WriteColor, value: &Value) -> Result<()> {
    write_json5(w, 0, value)?;
    writeln!(w)?;
    Ok(())
}

pub fn csv(w: &mut impl WriteColor, value: &Value) -> Result<()> {
    let rows = value
        .as_array()
        .context("expected array")?
        .iter()
        .map(|row| row.as_object().context("expected object"))
        .collect::<Result<Vec<_>>>()?;

    if rows.is_empty() {
        writeln!(w)?;
        return Ok(());
    }

    let header: IndexSet<_> = rows.iter().flat_map(|&row| row.keys()).collect();

    let mut writer = csv::Writer::from_writer(w);
    writer.write_record(&header)?;
    for row in rows {
        writer.write_record(header.iter().map(|&col| {
            row.get(col).map_or(String::new(), |v| match v {
                // Write strings as they are, the CSV writer will take care of quoting...
                Value::String(s) => s.to_owned(),
                // ...but serialise anything else to a string.
                _ => v.to_string(),
            })
        }))?;
    }
    writer.flush()?;

    Ok(())
}

pub fn error(w: &mut impl WriteColor, err: &Error) -> Result<()> {
    with_color(w, &ERR, |w| write!(w, "error"))?;
    writeln!(w, ": {err:#}")?;
    Ok(())
}

pub fn stdout() -> StandardStream {
    StandardStream::stdout(color_choice(&std::io::stdout()))
}

pub fn stderr() -> StandardStream {
    StandardStream::stderr(color_choice(&std::io::stderr()))
}

/// Recursively sort object keys so that objects print with keys in sorted order. Relies on the
/// `preserve_order` feature of Serde JSON.
pub fn sort(value: &Value) -> Value {
    match value {
        Value::Array(arr) => Value::Array(arr.iter().map(sort).collect()),
        Value::Object(obj) => {
            let mut keys: Vec<_> = obj.keys().collect();
            keys.sort_unstable();
            Value::Object(keys.iter().map(|&k| (k.clone(), sort(&obj[k]))).collect())
        }
        _ => value.clone(),
    }
}
