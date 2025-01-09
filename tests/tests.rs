use std::{
    env::{self, consts::EXE_SUFFIX},
    io::Write,
    process::{Command, Stdio},
};

use anyhow::{Context, Result};

#[derive(PartialEq, Debug)]
struct Output {
    status_code: i32,
    stdout: String,
    stderr: String,
}

fn run<'a, I>(args: &[&str], stdin: Option<&str>, vars: I) -> Result<Output>
where
    I: IntoIterator<Item = (&'a str, &'a str)>,
{
    let bin = env::current_exe()?
        .parent()
        .context("getting parent")?
        .parent()
        .context("getting parent")?
        .join(format!("fn{}", EXE_SUFFIX));

    let mut cmd = Command::new(bin);
    cmd.args(args);
    cmd.envs(vars);
    cmd.stdout(Stdio::piped());
    cmd.stderr(Stdio::piped());
    if stdin.is_some() {
        cmd.stdin(Stdio::piped());
    }

    let mut child = cmd.spawn()?;
    if let Some(stdin) = stdin {
        child
            .stdin
            .take()
            .context("getting stdin")?
            .write_all(stdin.as_bytes())?;
    }

    let output = child.wait_with_output()?;

    Ok(Output {
        status_code: output.status.code().context("getting status code")?,
        stdout: String::from_utf8(output.stdout)?,
        stderr: String::from_utf8(output.stderr)?,
    })
}

fn ok(stdout: &str) -> Output {
    Output {
        status_code: 0,
        stdout: String::from(stdout),
        stderr: String::new(),
    }
}

fn err(stderr: &str) -> Output {
    Output {
        status_code: 1,
        stdout: String::new(),
        stderr: String::from(stderr),
    }
}

#[test]
fn test() -> Result<()> {
    assert!(run(&[], None, [])?
        .stderr
        .starts_with("Evaluate a JavaScript function and print the result"));

    assert_eq!(run(&["2 + 3"], None, [])?, ok("5\n"));

    assert_eq!(
        run(&["{ const x = 5; return x * x }"], None, [])?,
        ok("25\n")
    );

    assert_eq!(run(&["-s", "undefined"], None, [])?, ok("undefined\n"));

    assert_eq!(
        run(&["-ps", "$.foo"], Some(r#"{ "foo": 42 }"#), [])?,
        ok("42\n")
    );

    assert_eq!(
        run(&["-ps", "$.foo"], Some(r#"{ "foo": "bar" }"#), [])?,
        ok("\"bar\"\n")
    );

    assert_eq!(
        run(
            &["-ps", "$.foo"],
            Some(r#"{ "foo": { "bar": [0, 1, 2] } }"#),
            []
        )?,
        ok("{\n  \"bar\": [\n    0,\n    1,\n    2\n  ]\n}\n")
    );

    assert_eq!(
        run(&["-s", "({ a: {}, b: [] })"], None, [])?,
        ok("{\n  \"a\": {},\n  \"b\": []\n}\n")
    );

    assert_eq!(run(&["$foo"], None, [("foo", "42")])?, ok("42\n"));

    assert_eq!(
        run(&[r"$.match(/foo:(\w*)/)[1]"], Some("foo:bar baz:42"), [])?,
        ok("bar\n")
    );

    assert_eq!(
        run(&["foo"], None, [])?,
        err("error: evaluating function: ReferenceError: foo is not defined\n")
    );

    assert_eq!(
        run(&["const x = 5; return x * x"], None, [])?,
        err("error: compiling script: SyntaxError: Unexpected token 'const'\n")
    );

    assert_eq!(
        run(&["-p"], Some("foo"), [])?,
        err("error: parsing STDIN: SyntaxError: Unexpected token 'o', \"foo\" is not valid JSON\n")
    );

    Ok(())
}
