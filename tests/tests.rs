use std::{
    env::{self, consts::EXE_SUFFIX},
    io::Write,
    process::{Command, Stdio},
};

use anyhow::{Context, Result, ensure};

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
        .join(format!("jsq{}", EXE_SUFFIX));

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

macro_rules! assert_ok {
    ($actual:expr, $expected:expr $(,)?) => {{
        let actual = $actual;
        assert_eq!(
            actual.status_code, 0,
            "status_code: {}\nstdout: {}\nstderr: {}",
            actual.status_code, actual.stdout, actual.stderr,
        );
        assert_eq!(
            actual.stdout, $expected,
            "status_code: {}\nstdout: {}\nstderr: {}",
            actual.status_code, actual.stdout, actual.stderr,
        );
    }};
}

macro_rules! assert_err {
    ($actual:expr, $expected:expr $(,)?) => {{
        let actual = $actual;
        assert_eq!(
            actual.status_code, 1,
            "status_code: {}\nstdout: {}\nstderr: {}",
            actual.status_code, actual.stdout, actual.stderr,
        );
        assert_eq!(
            actual.stdout, "",
            "status_code: {}\nstdout: {}\nstderr: {}",
            actual.status_code, actual.stdout, actual.stderr,
        );
        assert!(
            actual.stderr.contains($expected),
            "status_code: {}\nstdout: {}\nstderr: {}",
            actual.status_code,
            actual.stdout,
            actual.stderr,
        );
    }};
}

#[test]
fn test() -> Result<()> {
    let cargo_toml = include_str!("../Cargo.toml").replace("\r\n", "\n");
    let publish_yaml = include_str!("../.github/workflows/publish.yaml").replace("\r\n", "\n");

    assert!(
        run(&[], "", [])?
            .stderr
            .starts_with("Evaluate some JavaScript and print the result")
    );

    assert_ok!(run(&["2 + 3"], "", [])?, "5\n");

    assert_ok!(run(&["const x = 5; x * x"], "", [])?, "25\n");

    assert_ok!(run(&["-jJ", "$.foo"], r#"{ "foo": 42 }"#, [])?, "42\n");

    assert_ok!(
        run(&["-jJ", "$.foo"], r#"{ "foo": "bar" }"#, [])?,
        "\"bar\"\n"
    );

    assert_ok!(
        run(&["-jJ", "$.foo"], r#"{ "foo": { "bar": [0, 1, 2] } }"#, [])?,
        "{\n  \"bar\": [\n    0,\n    1,\n    2\n  ]\n}\n"
    );

    assert_ok!(
        run(&["-J", "({ a: {}, b: [] })"], "", [])?,
        "{\n  \"a\": {},\n  \"b\": []\n}\n"
    );

    assert_ok!(run(&["$foo"], "", [("foo", "42")])?, "42\n");

    assert_ok!(
        run(&[r"$.match(/foo:(\w*)/)[1]"], "foo:bar baz:42", [])?,
        "bar\n"
    );

    assert_err!(run(&["foo"], "", [])?, "ReferenceError: foo is not defined");

    assert_err!(
        run(&["return 42"], "", [])?,
        "A 'return' statement can only be used within a function body",
    );

    assert_err!(
        run(&["-j"], "foo", [])?,
        "parsing JSON: expected ident at line 1 column 2",
    );

    assert_ok!(
        run(&["-y", "$.jobs.info['runs-on']"], &publish_yaml, [])?,
        "macos-latest\n",
    );

    assert_ok!(run(&["-yY"], &publish_yaml, [])?, publish_yaml);

    assert_ok!(run(&["-t", "$.package.name"], &cargo_toml, [])?, "jsq\n");

    assert_ok!(run(&["-tT"], &cargo_toml, [])?, cargo_toml);

    assert_ok!(run(&["-J", "undefined"], "", [])?, "undefined\n");
    assert_ok!(run(&["-Y", "undefined"], "", [])?, "undefined\n");
    assert_ok!(run(&["-T", "undefined"], "", [])?, "undefined\n");
    assert_ok!(run(&["-J", "() => {}"], "", [])?, "undefined\n");
    assert_ok!(run(&["-Y", "() => {}"], "", [])?, "undefined\n");
    assert_ok!(run(&["-T", "() => {}"], "", [])?, "undefined\n");

    assert_eq!(
        convert("-tY", &convert("-jT", &convert("-yJ", &publish_yaml)?)?)?,
        publish_yaml
    );

    assert_eq!(
        convert("-jY", &convert("-tJ", &convert("-yT", &publish_yaml)?)?)?,
        publish_yaml
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

    assert_ok!(
        run(
            &[r#"
            import * as yaml from "jsr:@std/yaml";
            yaml.stringify(yaml.parse($).defaults)
            "#],
            &publish_yaml,
            []
        )?,
        "run:\n  shell: bash\n\n",
    );

    assert_ok!(
        run(
            &[r#"
            import * as toml from "jsr:@std/toml";
            toml.stringify(toml.parse($).dependencies.serde_json)
            "#],
            &cargo_toml,
            []
        )?,
        "version = \"1.0.145\"\nfeatures = [\"preserve_order\"]\n\n",
    );

    assert_ok!(
        run(
            &[r#"
            import * as toml from "jsr:@std/toml";
            toml.parse(Deno.readTextFileSync("Cargo.toml")).package.name
            "#],
            "",
            []
        )?,
        "jsq\n",
    );

    assert_ok!(
        run(&["-N", r#"console.log("foo"); console.log(42)"#], "", [])?,
        "foo\n42\n",
    );

    assert_ok!(
        run(&["-f", "tests/test.js"], "", [])?,
        "0\n1\n2\n3\n4\n42\n",
    );

    assert_ok!(run(&["let x"], "", [])?, "undefined\n");

    Ok(())
}
