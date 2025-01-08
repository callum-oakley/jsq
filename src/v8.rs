use anyhow::{anyhow, Context, Result};

macro_rules! with_catch {
    ($scope:expr, $option:expr) => {{
        let res = $option;
        if let Some(exception) = $scope.exception() {
            Err(anyhow!(exception.to_rust_string_lossy(&mut $scope)))
        } else {
            res.context("no exception but empty result")
        }
    }};
}

pub struct Options<'a, I> {
    pub body: &'a str,
    pub env: I,
    pub parse: bool,
    pub stdin: Option<String>,
    pub stringify: bool,
}

pub fn eval<I: Iterator<Item = (String, String)>>(options: Options<'_, I>) -> Result<String> {
    v8::V8::set_flags_from_string("--use-strict");
    let platform = v8::new_default_platform(0, false).make_shared();
    v8::V8::initialize_platform(platform);
    v8::V8::initialize();
    let mut isolate = v8::Isolate::new(v8::CreateParams::default());
    let mut scope = v8::HandleScope::new(&mut isolate);

    let object_template = v8::ObjectTemplate::new(&mut scope);
    for (k, v) in options.env {
        object_template.set(
            string(&mut scope, &format!("${k}"))?.into(),
            string(&mut scope, &v)?.into(),
        );
    }

    let context = v8::Context::new(
        &mut scope,
        v8::ContextOptions {
            global_template: Some(object_template),
            ..v8::ContextOptions::default()
        },
    );
    let mut scope = v8::ContextScope::new(&mut scope, context);
    let mut scope = v8::TryCatch::new(&mut scope);

    let undefined = v8::undefined(&mut scope);

    let stdin: v8::Local<v8::Value> = if let Some(s) = options.stdin {
        let s = string(&mut scope, &s)?;
        if options.parse {
            with_catch!(scope, v8::json::parse(&mut scope, s)).context("parsing STDIN")?
        } else {
            s.into()
        }
    } else {
        undefined.into()
    };

    let script = string(&mut scope, &format!("$ => {}", options.body))?;
    let script = with_catch!(scope, v8::Script::compile(&mut scope, script, None))
        .context("compiling script")?;

    let f = with_catch!(scope, script.run(&mut scope))
        .context("running script")?
        .try_cast::<v8::Function>()?;

    let mut res = with_catch!(scope, f.call(&mut scope, undefined.into(), &[stdin]))
        .context("evaluating function")?;

    if options.stringify {
        res = with_catch!(scope, v8::json::stringify(&mut scope, res))
            .context("stringifying JSON")?
            .into();
    }

    Ok(res.to_rust_string_lossy(&mut scope))
}

fn string<'s>(scope: &mut v8::HandleScope<'s, ()>, s: &str) -> Result<v8::Local<'s, v8::String>> {
    v8::String::new(scope, s).context("constructing string")
}
