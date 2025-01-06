# fn

fn is a tool for evaluating a JavaScript function and printing the result.

## Help

```
Evaluate a JavaScript function and print the result

Usage: fn [OPTIONS] [BODY]

Arguments:
  [BODY]  The body of the JavaScript function to be evaluated [default: $]

Options:
  -p, --parse      JSON.parse STDIN before passing it to the function
  -s, --stringify  JSON.stringify the result before printing it to STDOUT
  -h, --help       Print help
  -V, --version    Print version

STDIN is avaialable in BODY as $. Environment variables are available in BODY prefixed by $.
```

## Example

Suppose we have some JSON which contains [a bunch of superheros][] and we want to find the hero with
the power of "Immortality":

```
> curl https://mdn.github.io/learning-area/javascript/oojs/json/superheroes.json |
    fn -ps '$.members.find(m => m.powers.includes("Immortality"))'
{
  "name": "Eternal Flame",
  "age": 1000000,
  "secretIdentity": "Unknown",
  "powers": [
    "Immortality",
    "Heat Immunity",
    "Inferno",
    "Teleportation",
    "Interdimensional travel"
  ]
}
```

## Semantics

The provided `BODY` is evaluated by [V8][] as part of the expression `(() => BODY)()`. In
particular, this means that `BODY` must have the syntax of an [Arrow Function][] body: it can either
be a single expression, or multiple statements enclosed in braces with an explicit return statement.

If STDIN is not a terminal then `$` contains the result of reading STDIN as text, or of parsing it
as JSON if the `-p` flag is set. If STDIN [is a terminal][] then `$` is not defined.

The result is printed to STDOUT after being [cast to a string][], or serialized as JSON if the `-s`
flag is set.

Environment variables are available in `BODY` prefixed by `$`. e.g. `USER` is available as `$USER`.

## Why?

JavaScript is a convenient language to use to process JSON (which stands for "JavaScript Object
Notation" after all), but the boilerplate of reading from STDIN, parsing, and writing to STDOUT
makes many could-be "one-liners" significantly more involved than they need to be. fn provides a
thin wrapper around V8 which handles this boilerplate and makes it more ergonomic to sprinkle a
little JavaScript in to a shell script.

fn can be used for many of the same tasks as [jq][]. A given jq command is often a little shorter
than the equivalent fn command, but if (like the author) you find yourself often forgetting the
syntax of jq, and you already know JavaScript, you might find fn easier to use. To see how fn
compares to jq, check out the [translated jq tutorial][].

## Install

First you'll need to [install Rust][], then

```
cargo install fn
```

[a bunch of superheros]: https://mdn.github.io/learning-area/javascript/oojs/json/superheroes.json
[Arrow Function]: https://developer.mozilla.org/en-US/docs/Web/JavaScript/Reference/Functions/Arrow_functions
[cast to a string]: https://developer.mozilla.org/en-US/docs/Web/JavaScript/Reference/Global_Objects/Object/toString
[install Rust]: https://www.rust-lang.org/tools/install
[is a terminal]: https://doc.rust-lang.org/beta/std/io/trait.IsTerminal.html#tymethod.is_terminal
[jq]: https://jqlang.github.io/jq/
[translated jq tutorial]: /tutorial.md
[V8]: https://v8.dev/
