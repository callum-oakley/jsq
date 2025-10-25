use std::fmt::Write as _;
use std::io::Write as _;
use std::process::{Command, Output, Stdio};

use anyhow::{Context, Result, anyhow, bail, ensure};
use oxc::allocator::{Allocator, TakeIn};
use oxc::ast::ast::{Program, Statement};
use oxc::ast::{AstBuilder, ast::Expression};
use oxc::ast_visit::VisitMut;
use oxc::codegen::Codegen;
use oxc::parser::Parser;
use oxc::span::{SourceType, Span};

#[derive(Copy, Clone)]
pub enum Print {
    None,
    String,
    Object,
}

pub struct Options<'a, I> {
    pub input: &'a str,
    pub env: I,
    pub script: &'a str,
    pub parse: bool,
    pub print: Print,
}

pub fn eval<I: Iterator<Item = (String, String)>>(options: Options<'_, I>) -> Result<Output> {
    let allocator = Allocator::new();

    let mut program = parse(&allocator, options.script)?;

    program.body.insert(
        0,
        sub_undefined(
            &allocator,
            if options.parse {
                "const $ = JSON.parse(undefined);"
            } else {
                "const $ = undefined;"
            },
            string_literal(&allocator, options.input),
        )?,
    );

    for (k, v) in options.env {
        // Ignore weird environment variable names.
        if k.chars().all(|c| c.is_alphanumeric() || c == '_') {
            program.body.insert(
                0,
                sub_undefined(
                    &allocator,
                    AstBuilder::new(&allocator).str(&format!("const ${k} = undefined;")),
                    string_literal(&allocator, AstBuilder::new(&allocator).str(&v)),
                )?,
            );
        }
    }

    if !matches!(options.print, Print::None) {
        let statement = program.body.pop().expect("program is not empty");
        if let Statement::ExpressionStatement(mut expression_statement) = statement {
            program.body.push(sub_undefined(
                &allocator,
                match options.print {
                    Print::String => "console.log(undefined);",
                    Print::Object => "console.log(JSON.stringify(undefined));",
                    _ => unreachable!(),
                },
                expression_statement.expression.take_in(&allocator),
            )?);
        } else {
            // Final statement isn't an expression statement so result is undefined.
            program.body.push(statement);
            program
                .body
                .push(parse_statment(&allocator, "console.log(undefined);")?);
        }
    }

    let code = Codegen::new().build(&program).code;

    let mut child = Command::new("deno")
        .arg("run")
        .arg("--allow-all")
        .arg("-")
        .stdin(Stdio::piped())
        .stdout(match options.print {
            Print::Object => Stdio::piped(),
            _ => Stdio::inherit(),
        })
        .spawn()
        .map_err(|err| {
            if err.kind() == std::io::ErrorKind::NotFound {
                anyhow!("command not found: deno")
            } else {
                err.into()
            }
        })?;

    // Write code to child's STDIN from another thread to avoid potential deadlock. From
    // https://doc.rust-lang.org/std/process/struct.Stdio.html#method.piped:
    //
    // > Writing more than a pipe buffer’s worth of input to stdin without also reading stdout and
    // > stderr at the same time may cause a deadlock. This is an issue when running any program
    // > that doesn’t guarantee that it reads its entire stdin before writing more than a pipe
    // > buffer’s worth of output. The size of a pipe buffer varies on different targets.
    let mut cmdin = child.stdin.take().context("opening stdin")?;
    std::thread::spawn(move || {
        cmdin.write_all(code.as_bytes()).expect("writing to stdin");
    });

    Ok(child.wait_with_output()?)
}

fn parse<'a>(allocator: &'a Allocator, s: &'a str) -> Result<Program<'a>> {
    let res = Parser::new(allocator, s, SourceType::ts()).parse();
    if !res.errors.is_empty() {
        let mut msg = String::from("parsing script:");
        for err in res.errors {
            msg.push_str("\n  - ");
            write!(&mut msg, "{err}")?;
        }
        bail!(msg);
    }
    Ok(res.program)
}

fn parse_statment<'a>(allocator: &'a Allocator, s: &'a str) -> Result<Statement<'a>> {
    let mut program = parse(allocator, s)?;
    ensure!(program.body.len() == 1);
    Ok(program.body.swap_remove(0))
}

fn sub_undefined<'a>(
    allocator: &'a Allocator,
    template: &'a str,
    expression: Expression<'a>,
) -> Result<Statement<'a>> {
    struct ReplaceUndefined<'a> {
        expression: Option<Expression<'a>>,
    }

    impl<'a> VisitMut<'a> for ReplaceUndefined<'a> {
        fn visit_expression(&mut self, it: &mut Expression<'a>) {
            if self.expression.is_some() && it.is_undefined() {
                *it = self.expression.take().unwrap();
            } else {
                oxc::ast_visit::walk_mut::walk_expression(self, it);
            }
        }
    }

    let mut statement = parse_statment(allocator, template)?;

    oxc::ast_visit::walk_mut::walk_statement(
        &mut ReplaceUndefined {
            expression: Some(expression),
        },
        &mut statement,
    );

    Ok(statement)
}

fn string_literal<'a>(allocator: &'a Allocator, s: &'a str) -> Expression<'a> {
    Expression::StringLiteral(AstBuilder::new(allocator).alloc_string_literal(
        Span::new(0, 0),
        s,
        None,
    ))
}
