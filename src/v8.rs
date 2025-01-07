use std::collections::HashMap;

use anyhow::{bail, Context, Result};

fn string<'s>(scope: &mut v8::HandleScope<'s, ()>, s: &str) -> Result<v8::Local<'s, v8::String>> {
    v8::String::new(scope, s).context("constructing string")
}

pub fn eval(script: &str, vars: &HashMap<String, String>) -> Result<String> {
    let platform = v8::new_default_platform(0, false).make_shared();
    v8::V8::initialize_platform(platform);
    v8::V8::initialize();
    let mut isolate = v8::Isolate::new(v8::CreateParams::default());

    let mut scope = v8::HandleScope::new(&mut isolate);
    let object_template = v8::ObjectTemplate::new(&mut scope);
    for (k, v) in vars {
        object_template.set(string(&mut scope, k)?.into(), string(&mut scope, v)?.into());
    }
    let context = v8::Context::new(
        &mut scope,
        v8::ContextOptions {
            global_template: Some(object_template),
            global_object: None,
            microtask_queue: None,
        },
    );
    let mut scope = v8::ContextScope::new(&mut scope, context);
    let mut scope = v8::TryCatch::new(&mut scope);

    let script = string(&mut scope, script)?;
    let script = v8::Script::compile(&mut scope, script, None);
    if let Some(exception) = scope.exception() {
        bail!("{}", exception.to_rust_string_lossy(&mut scope))
    }

    let res = script.context("compiling script")?.run(&mut scope);
    if let Some(exception) = scope.exception() {
        bail!("{}", exception.to_rust_string_lossy(&mut scope))
    }

    Ok(res
        .context("running script")?
        .to_rust_string_lossy(&mut scope))
}
