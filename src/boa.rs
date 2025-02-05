use anyhow::{anyhow, Result};
use boa_engine::{property::Attribute, Context, JsResult, JsString, JsValue, Source};

pub struct Options<'a, I> {
    pub input: &'a str,
    pub env: I,
    pub body: &'a str,
    pub parse: bool,
    pub stringify: bool,
}

trait ContextExt<T> {
    fn context(self, context: &str) -> Result<T>;
}

impl<T> ContextExt<T> for JsResult<T> {
    fn context(self, context: &str) -> Result<T> {
        self.map_err(|err| anyhow!("{context}: {err}"))
    }
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
        .context("registering global property")?;

    if options.parse {
        context
            .eval(Source::from_bytes("$ = JSON.parse($)"))
            .context("parsing input as JSON")?;
    }

    for (k, v) in options.env {
        context
            .register_global_property(
                JsString::from(format!("${k}")),
                JsString::from(v),
                Attribute::all(),
            )
            .context("registering global property")?;
    }

    let mut source = format!("(() => {})()", options.body);
    if options.stringify {
        source = format!("JSON.stringify({source})");
    }

    let res = context
        .eval(Source::from_bytes(&source))
        .context("evaluating function")?;

    Ok(res
        .to_string(&mut context)
        .context("converting to string")?
        .to_std_string()?)
}
