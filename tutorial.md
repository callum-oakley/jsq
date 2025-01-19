# Tutorial

jfn can be used for a lot of the same tasks as [jq](https://jqlang.github.io/jq/). Below is a copy of
the [jq tutorial](https://jqlang.github.io/jq/tutorial/) with all the jq translated to jfn.

---

GitHub has a JSON API, so let's play with that. This URL gets us the last 5 commits from the jfn
repo.

```
curl 'https://api.github.com/repos/callum-oakley/jfn/commits?per_page=5'
```

GitHub returns nicely formatted JSON. For servers that don't, it can be helpful to pipe the response
through jfn to pretty-print it. The `-j` flag parses the input as JSON before passing it to the
function, and the `-J` flag prints the output as JSON. The default function body is the identity:
`$`.

```
curl 'https://api.github.com/repos/callum-oakley/jfn/commits?per_page=5' | jfn -jJ
```

We can use jfn to extract just the first commit. `$` is the result of parsing STDIN as JSON, so
`$[0]` is the first commit.

```
curl 'https://api.github.com/repos/callum-oakley/jfn/commits?per_page=5' | jfn -jJ '$[0]'
```

For the rest of the examples, I'll leave out the `curl` command - it's not going to change.

There's a lot of info we don't care about there, so we'll restrict it down to the most interesting
fields.

```
jfn -jJ '{ const a = $[0]; return { message: a.commit.message, name: a.commit.committer.name } }'
```

We assign `$[0]` to a variable and then use that variable to construct a new object with only the
fields we care about. Note that we're using two statements here, so just like in a JavaScript arrow
function, we need to wrap the body in braces and use an explicit `return` statement.

Now let's get the rest of the commits by applying the same transformation to every commit with
`map`.

```
jfn -jJ '$.map(a => ({ message: a.commit.message, name: a.commit.committer.name }))'
```

Next, let's try getting the URLs of the parent commits out of the API results as well. In each
commit, the GitHub API includes information about "parent" commits. There can be one or many.

```
"parents": [
  {
    "sha": "66ec47a4f84d3ab2dfe95595a57461fa4d19f8d6",
    "url": "https://api.github.com/repos/callum-oakley/jfn/commits/66ec47a4f84d3ab2dfe95595a57461fa4d19f8d6",
    "html_url": "https://github.com/callum-oakley/jfn/commit/66ec47a4f84d3ab2dfe95595a57461fa4d19f8d6"
  }
]
```

We want to pull out all of the "html_url" fields inside that array of parent commits and make a
simple list of strings to go along with the "message" and "author" fields we already have.

```
jfn -jJ '$.map(a => ({
  message: a.commit.message,
  name: a.commit.committer.name,
  parents: a.parents.map(b => b.html_url),
}))'
```

Here we're making a new object for each commit as before, but this time we use another nested `map`
to pull the commit URLs out of each parent object.

---

Here endeth the tutorial! There's lots more to play with. [Install jfn](/README.md) if you haven't
already, and check out `jfn --help` to see all the available options.
