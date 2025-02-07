use anyhow::{Context as _, Error, Result};
use boa_engine::{
    object::ObjectInitializer, property::Attribute, Context, JsArgs, JsError, JsResult, JsString,
    JsValue, NativeFunction, Source,
};
use serde_json::Value;

pub struct Options<'a, I> {
    pub input: &'a str,
    pub env: I,
    pub body: &'a str,
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

macro_rules! register_parse_and_stringify {
    ($name:expr, $serde:ident, $context:expr) => {{
        let obj = ObjectInitializer::new($context)
            .function(
                NativeFunction::from_fn_ptr(|_, args, context| {
                    let s = args
                        .get_or_undefined(0)
                        .to_string(context)?
                        .to_std_string()
                        .to_js()?;
                    let value = $serde::from_str::<Value>(&s).to_js()?;
                    call_fn(
                        "JSON.parse",
                        &[JsValue::from(JsString::from(
                            serde_json::to_string(&value).to_js()?,
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
                    let s = call_fn("JSON.stringify", args, context)
                        .to_js()?
                        .to_string(context)?
                        .to_std_string()
                        .to_js()?;
                    let value = serde_json::from_str::<Value>(&s).to_js()?;
                    Ok(JsValue::from(JsString::from(
                        // TODO use print?
                        $serde::to_string(&value).to_js()?,
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

    register_parse_and_stringify!("YAML", serde_yaml, &mut context);
    register_parse_and_stringify!("TOML", toml, &mut context);

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
        .eval(Source::from_bytes(&format!("(() => {})()", options.body)))
        .to_anyhow(&mut context)?;

    if options.stringify {
        res = call_fn("JSON.stringify", &[res], &mut context)?;
    }

    Ok(res
        .to_string(&mut context)
        .to_anyhow(&mut context)?
        .to_std_string()?)
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
