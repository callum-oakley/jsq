use std::{
    env::{self, consts::EXE_SUFFIX},
    io::Write,
    process::{Command, Stdio},
};

use anyhow::{ensure, Context, Result};

#[derive(PartialEq, Debug)]
struct Output {
    status_code: i32,
    stdout: String,
    stderr: String,
}

fn run<'a, I>(args: &[&str], stdin: &str, vars: I) -> Result<Output>
where
    I: IntoIterator<Item = (&'a str, &'a str)>,
{
    let bin = env::current_exe()?
        .parent()
        .context("getting parent")?
        .parent()
        .context("getting parent")?
        .join(format!("jfn{}", EXE_SUFFIX));

    let mut child = Command::new(bin)
        .args(args)
        .envs(vars)
        .stdin(Stdio::piped())
        .stdout(Stdio::piped())
        .stderr(Stdio::piped())
        .spawn()?;

    child
        .stdin
        .take()
        .context("getting stdin")?
        .write_all(stdin.as_bytes())?;

    let output = child.wait_with_output()?;

    Ok(Output {
        status_code: output.status.code().context("getting status code")?,
        stdout: String::from_utf8(output.stdout)?,
        stderr: String::from_utf8(output.stderr)?,
    })
}

fn convert(flags: &str, stdin: &str) -> Result<String> {
    let res = run(&[flags], stdin, [])?;
    ensure!(res.status_code == 0);
    ensure!(res.stderr == "");
    Ok(res.stdout)
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
    let cargo_toml = include_str!("../Cargo.toml").replace("\r\n", "\n");
    let publish_yml = include_str!("../.github/workflows/publish.yml").replace("\r\n", "\n");

    assert!(run(&[], "", [])?
        .stderr
        .starts_with("Evaluate a JavaScript function and print the result"));

    assert_eq!(run(&["2 + 3"], "", [])?, ok("5\n"));

    assert_eq!(run(&["{ const x = 5; return x * x }"], "", [])?, ok("25\n"));

    assert_eq!(run(&["-jJ", "$.foo"], r#"{ "foo": 42 }"#, [])?, ok("42\n"));

    assert_eq!(
        run(&["-jJ", "$.foo"], r#"{ "foo": "bar" }"#, [])?,
        ok("\"bar\"\n")
    );

    assert_eq!(
        run(&["-jJ", "$.foo"], r#"{ "foo": { "bar": [0, 1, 2] } }"#, [])?,
        ok("{\n  \"bar\": [\n    0,\n    1,\n    2\n  ]\n}\n")
    );

    assert_eq!(
        run(&["-J", "({ a: {}, b: [] })"], "", [])?,
        ok("{\n  \"a\": {},\n  \"b\": []\n}\n")
    );

    assert_eq!(run(&["$foo"], "", [("foo", "42")])?, ok("42\n"));

    assert_eq!(
        run(&[r"$.match(/foo:(\w*)/)[1]"], "foo:bar baz:42", [])?,
        ok("bar\n")
    );

    assert_eq!(
        run(&["foo"], "", [])?,
        err("error: evaluating function: ReferenceError: foo is not defined\n")
    );

    assert_eq!(
        run(&["const x = 5; return x * x"], "", [])?,
        err("error: compiling script: SyntaxError: Unexpected token 'const'\n")
    );

    assert_eq!(
        run(&["-j"], "foo", [])?,
        err("error: parsing JSON: SyntaxError: Unexpected token 'o', \"foo\" is not valid JSON\n")
    );

    assert_eq!(
        run(
            &["-y", "$.jobs['get-version']['runs-on']"],
            &publish_yml,
            []
        )?,
        ok("ubuntu-latest\n")
    );

    assert_eq!(run(&["-yY"], &publish_yml, [])?, ok(&publish_yml));

    assert_eq!(
        run(&["-t", "$.package.name"], &cargo_toml, [])?,
        ok("jfn\n")
    );

    assert_eq!(run(&["-tT"], &cargo_toml, [])?, ok(&cargo_toml));

    assert_eq!(run(&["-J", "undefined"], "", [])?, ok("undefined\n"));
    assert_eq!(run(&["-Y", "undefined"], "", [])?, ok("undefined\n"));
    assert_eq!(run(&["-T", "undefined"], "", [])?, ok("undefined\n"));

    assert_eq!(
        convert("-tY", &convert("-jT", &convert("-yJ", &publish_yml)?)?)?,
        publish_yml
    );

    assert_eq!(
        convert("-jY", &convert("-tJ", &convert("-yT", &publish_yml)?)?)?,
        publish_yml
    );

    assert_eq!(
        convert("-yT", &convert("-jY", &convert("-tJ", &cargo_toml)?)?)?,
        cargo_toml
    );

    assert_eq!(
        convert("-jT", &convert("-yJ", &convert("-tY", &cargo_toml)?)?)?,
        cargo_toml
    );

    assert_eq!(convert("-jY", "{ \"foo\": \"true\" }")?, "foo: \"true\"\n");

    Ok(())
}
