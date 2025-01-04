# Tutorial

fn can be used for a lot of the same tasks as [jq](https://jqlang.github.io/jq/). Below is a copy of
the [jq tutorial](https://jqlang.github.io/jq/tutorial/) with all the jq translated to fn.

---

GitHub has a JSON API, so let's play with that. This URL gets us the last 5 commits from the fn
repo.

```
curl 'https://api.github.com/repos/callum-oakley/fn/commits?per_page=5'
```

GitHub returns nicely formatted JSON. For servers that don't, it can be helpful to pipe the response
through fn to pretty-print it. The `-p` flag calls `JSON.parse` on STDIN before passing it to the
function, and the `-s` flag calls `JSON.stringify` on the result before printing it to STDOUT. The
default function body is the identity: `$`.

```
curl 'https://api.github.com/repos/callum-oakley/fn/commits?per_page=5' | fn -ps
```

We can use fn to extract just the first commit. `$` is the result of parsing STDIN as JSON, so
`$[0]` is the first commit.

```
curl 'https://api.github.com/repos/callum-oakley/fn/commits?per_page=5' | fn -ps '$[0]'
```

For the rest of the examples, I'll leave out the `curl` command - it's not going to change.

There's a lot of info we don't care about there, so we'll restrict it down to the most interesting
fields.

```
fn -ps '{ const a = $[0]; return { message: a.commit.message, name: a.commit.committer.name } }'
```

We assign `$[0]` to a variable and then use that variable to construct a new object with only the
fields we care about. Note that we're using two statements here, so just like in a JavaScript arrow
function, we need to wrap the body in braces and use an explicit `return` statement.

Now let's get the rest of the commits by applying the same transformation to every commit with
`map`.

```
fn -ps '$.map(a => ({ message: a.commit.message, name: a.commit.committer.name }))'
```

Next, let's try getting the URLs of the parent commits out of the API results as well. In each
commit, the GitHub API includes information about "parent" commits. There can be one or many.

```
"parents": [
  {
    "sha": "4acd103768b307907f1d334eeed97674c732f067",
    "url": "https://api.github.com/repos/callum-oakley/fn/commits/4acd103768b307907f1d334eeed97674c732f067",
    "html_url": "https://github.com/callum-oakley/fn/commit/4acd103768b307907f1d334eeed97674c732f067"
  }
]
```

We want to pull out all of the "html_url" fields inside that array of parent commits and make a
simple list of strings to go along with the "message" and "author" fields we already have.

```
fn -ps '$.map(a => ({
  message: a.commit.message,
  name: a.commit.committer.name,
  parents: a.parents.map(b => b.html_url),
}))'
```

Here we're making a new object for each commit as before, but this time we use another nested `map`
to pull the commit URLs out of each parent object.

---

Here endeth the tutorial! There's lots more to play with. [Install fn](/README.md) if you haven't
already, and check out `fn --help` to see all the available options.
