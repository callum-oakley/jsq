# jsq

jsq is a tool for evaluating some JavaScript and printing the result.

## Help

```
Evaluate some JavaScript and print the result

Usage: jsq [OPTIONS] [SCRIPT]

Arguments:
  [SCRIPT]  The JavaScript to be evaluated [default: $]

Options:
  -j, --json-in      Parse input as JSON
  -y, --yaml-in      Parse input as YAML
  -t, --toml-in      Parse input as TOML
  -J, --json-out     Print result as JSON
  -Y, --yaml-out     Print result as YAML
  -T, --toml-out     Print result as TOML
  -N, --no-out       Don't print result
  -f, --file <FILE>  Read SCRIPT from FILE
  -h, --help         Print help
  -V, --version      Print version

Input is avaialable in SCRIPT as $. Environment variables are available in SCRIPT prefixed by $.
```

## Example

Suppose we have some JSON which contains [a bunch of superheros][] and we want to find the hero with
the power of "Immortality":

```
> curl https://mdn.github.io/learning-area/javascript/oojs/json/superheroes.json |
    jsq -jJ '$.members.find(m => m.powers.includes("Immortality"))'
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

The provided `SCRIPT` is evaluated by [Deno][] so the Deno runtime and standard library are
available, as are [third party imports][].

`$` contains the result of reading STDIN as text, or of parsing it as JSON if the `-j` flag is set,
YAML if the `-y` flag is set, or TOML if the `-t` flag is set. If STDIN [is a terminal][] then `$`
is the empty string.

The value of the final statement in `SCRIPT` (or `undefined` if the final statment is not an
expression statment) is printed to STDOUT – after being serialized as JSON if the `-J` flag is set,
YAML if the `-Y` flag is set, or TOML if the `-T` flag is set – unless the `-N` flag is set.

Environment variables are available in `SCRIPT` prefixed by `$`. e.g. `USER` is available as
`$USER`.

## Why?

JavaScript is a convenient language with which to process JSON (which stands for "JavaScript Object
Notation" after all), but the boilerplate of reading from STDIN, parsing, and writing to STDOUT
makes many could-be "one-liners" significantly more involved than they need to be. jsq provides a
thin wrapper around Deno which handles this boilerplate and makes it more ergonomic to sprinkle a
little JavaScript in to a shell script.

jsq can be used for many of the same tasks as [jq][]. A given jq command is often a little shorter
than the equivalent jsq command, but if (like the author) you find yourself often forgetting the
syntax of jq, and you already know JavaScript, you might find jsq easier to use. To see how jsq
compares to jq, check out the [translated jq tutorial][].

## Install

### With [brew][]

```
brew install callum-oakley/tap/jsq
```

### With [cargo][]

[Install Deno][] and then:

```
cargo install jsq
```

### Binaries

There are binaries for Linux, MacOS, and Windows [attached to each release][]. [Install Deno][],
download a binary, and make it available in your PATH.

[a bunch of superheros]: https://mdn.github.io/learning-area/javascript/oojs/json/superheroes.json
[attached to each release]: https://github.com/callum-oakley/jsq/releases
[brew]: https://brew.sh/
[cargo]: https://www.rust-lang.org/tools/install
[Deno]: https://deno.com/
[Install Deno]: https://docs.deno.com/runtime/getting_started/installation/
[is a terminal]: https://doc.rust-lang.org/beta/std/io/trait.IsTerminal.html#tymethod.is_terminal
[jq]: https://jqlang.github.io/jq/
[third party imports]: https://docs.deno.com/runtime/fundamentals/modules/#importing-third-party-modules-and-libraries
[translated jq tutorial]: /tutorial.md
