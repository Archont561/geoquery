---
title: The command line
description: The two subcommands the geoquery binary has today, and the exit codes they promise.
---

The `geoquery` binary is a presentation layer over `geoquery-core`: it parses arguments,
asks the library what a document is, prints the answer and picks an exit code. Every rule
about what a query *is* lives in the library, because a rule in the binary is a rule the
service would have to write again.

## `geoquery check <QUERY>`

Reads a JSON query document, reports what it contains, and contacts nothing.

```console
$ geoquery check query.json
query.json is a query object with 2 keys: execution, limit
```

An empty object is accepted. `{}` is a degenerate query, not a malformed one, and
refusing it here would mean having an opinion about what an empty query means before the
engine exists to have one.

## `geoquery query --service <URL> --query <PATH>`

Parses the document, then declines to execute it:

```console
$ geoquery query --service https://example.org --query query.json
geoquery: no engine yet — https://example.org was not contacted. geoquery 0.1.0 reads
query documents; executing them arrives with the query engine.
```

The document is checked *before* the refusal, because a malformed document is worth
reporting even when the request cannot be sent.

`--service` belongs to `query` and not to the top level. A service URL is exactly the
argument `check` must refuse to take: checking is the operation that reaches no network.

## Exit codes

These are the interface. They are asserted by the CLI's test suite, not documented and
hoped for.

| Code | Meaning |
| ---- | ------- |
| 0 | The document was read and reported. |
| 1 | The file could not be read — it is missing, or unreadable. |
| 2 | The file is not a query document — not JSON, or JSON that is not an object. |
| 3 | There is no engine yet. Nothing was sent. |

A missing file and a malformed file are deliberately different codes: one is a question
about the path, the other a question about the contents, and a script that retries the
first should not retry the second.

## The version it reports

`geoquery --version` prints the version of the *language*, not of the build. The two are
one number today, and `pixi run version-check` is what keeps every published manifest
agreeing on it.
