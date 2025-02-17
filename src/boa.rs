use std::fs::File;
use std::io::Write;

use anyhow::{Context as _, Error, Result};
use boa_engine::{
    object::ObjectInitializer, property::Attribute, Context, JsArgs, JsError, JsResult, JsString,
    JsValue, NativeFunction, Source,
};

use crate::{parse, print};

pub struct Options<'a, I> {
    pub input: &'a str,
    pub env: I,
    pub script: &'a str,
    pub parse: bool,
    pub stringify: bool,
}

trait ToAnyhow<T> {
    fn to_anyhow(self, context: &mut Context) -> Result<T>;
}

impl<T> ToAnyhow<T> for JsResult<T> {
    fn to_anyhow(self, context: &mut Context) -> Result<T> {
        self.map_err(|err| err.into_erased(context).into())
    }
}

trait ToJs<T> {
    fn to_js(self) -> JsResult<T>;
}

impl<T, E: Into<Error>> ToJs<T> for std::result::Result<T, E> {
    fn to_js(self) -> JsResult<T> {
        self.map_err(|err| JsError::from_rust(&*err.into()))
    }
}

fn call_fn(name: &str, args: &[JsValue], context: &mut Context) -> Result<JsValue> {
    context
        .eval(Source::from_bytes(name))
        .to_anyhow(context)?
        .as_callable()
        .context("as callable")?
        .call(&JsValue::undefined(), args, context)
        .to_anyhow(context)
}

fn get_std_string(args: &[JsValue], index: usize, context: &mut Context) -> JsResult<String> {
    args.get_or_undefined(index)
        .to_string(context)?
        .to_std_string()
        .to_js()
}

fn register_read(context: &mut Context) -> Result<()> {
    context
        .register_global_builtin_callable(
            JsString::from("read"),
            1,
            NativeFunction::from_fn_ptr(|_, args, context| {
                Ok(JsValue::from(JsString::from(
                    std::fs::read_to_string(get_std_string(args, 0, context)?).to_js()?,
                )))
            }),
        )
        .to_anyhow(context)
}

fn register_write(context: &mut Context) -> Result<()> {
    context
        .register_global_builtin_callable(
            JsString::from("write"),
            2,
            NativeFunction::from_fn_ptr(|_, args, context| {
                let mut file = File::create(get_std_string(args, 0, context)?).to_js()?;
                let value = get_std_string(args, 1, context)?;
                if value.ends_with('\n') {
                    write!(file, "{value}").to_js()?;
                } else {
                    writeln!(file, "{value}").to_js()?;
                }
                Ok(JsValue::Undefined)
            }),
        )
        .to_anyhow(context)
}

fn register_print(context: &mut Context) -> Result<()> {
    context
        .register_global_builtin_callable(
            JsString::from("print"),
            1,
            NativeFunction::from_fn_ptr(|_, args, context| {
                let value = get_std_string(args, 0, context)?;
                if value.ends_with('\n') {
                    print!("{value}");
                } else {
                    println!("{value}");
                }
                Ok(JsValue::Undefined)
            }),
        )
        .to_anyhow(context)
}

macro_rules! register_parse_and_stringify {
    ($name:expr, $parse:expr, $print:expr, $context:expr) => {{
        let obj = ObjectInitializer::new($context)
            .function(
                NativeFunction::from_fn_ptr(|_, args, context| {
                    call_fn(
                        "JSON.parse",
                        &[JsValue::from(JsString::from(
                            $parse(&get_std_string(args, 0, context)?).to_js()?,
                        ))],
                        context,
                    )
                    .to_js()
                }),
                JsString::from("parse"),
                1,
            )
            .function(
                NativeFunction::from_fn_ptr(|_, args, context| {
                    Ok(JsValue::from(JsString::from(
                        $print(
                            &call_fn("JSON.stringify", args, context)
                                .to_js()?
                                .to_string(context)?
                                .to_std_string()
                                .to_js()?,
                        )
                        .to_js()?,
                    )))
                }),
                JsString::from("stringify"),
                1,
            )
            .build();

        $context
            .register_global_property(JsString::from($name), obj, Attribute::all())
            .to_anyhow($context)?;
    }};
}

pub fn eval<I: Iterator<Item = (String, String)>>(options: Options<'_, I>) -> Result<String> {
    let mut context = Context::default();
    context.strict(true);

    register_read(&mut context)?;
    register_write(&mut context)?;
    register_print(&mut context)?;

    register_parse_and_stringify!("YAML", parse::yaml, print::yaml_to_string, &mut context);
    register_parse_and_stringify!("TOML", parse::toml, print::toml_to_string, &mut context);

    let mut input = JsValue::from(JsString::from(options.input));
    if options.parse {
        input = call_fn("JSON.parse", &[input], &mut context)?;
    }
    context
        .register_global_property(JsString::from("$"), input, Attribute::all())
        .to_anyhow(&mut context)?;

    for (k, v) in options.env {
        context
            .register_global_property(
                JsString::from(format!("${k}")),
                JsString::from(v),
                Attribute::all(),
            )
            .to_anyhow(&mut context)?;
    }

    let mut res = context
        .eval(Source::from_bytes(options.script))
        .to_anyhow(&mut context)?;

    if options.stringify {
        res = call_fn("JSON.stringify", &[res], &mut context)?;
    }

    Ok(res
        .to_string(&mut context)
        .to_anyhow(&mut context)?
        .to_std_string()?)
}
