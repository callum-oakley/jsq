use anyhow::Result;
use boa_engine::{property::Attribute, Context, JsString, JsValue, Source};

pub struct Options<'a, I> {
    pub input: &'a str,
    pub env: I,
    pub body: &'a str,
    pub parse: bool,
    pub stringify: bool,
}

pub fn eval<I: Iterator<Item = (String, String)>>(options: Options<'_, I>) -> Result<String> {
    let mut context = Context::default();
    context.strict(true);

    context
        .register_global_property(
            JsString::from("$"),
            JsValue::new(JsString::from(options.input)),
            Attribute::all(),
        )
        .map_err(|err| err.into_erased(&mut context))?;

    if options.parse {
        context
            .eval(Source::from_bytes("$ = JSON.parse($)"))
            .map_err(|err| err.into_erased(&mut context))?;
    }

    for (k, v) in options.env {
        context
            .register_global_property(
                JsString::from(format!("${k}")),
                JsString::from(v),
                Attribute::all(),
            )
            .map_err(|err| err.into_erased(&mut context))?;
    }

    let mut source = format!("(() => {})()", options.body);
    if options.stringify {
        source = format!("JSON.stringify({source})");
    }

    let res = context
        .eval(Source::from_bytes(&source))
        .map_err(|err| err.into_erased(&mut context))?;

    Ok(res
        .to_string(&mut context)
        .map_err(|err| err.into_erased(&mut context))?
        .to_std_string()?)
}
