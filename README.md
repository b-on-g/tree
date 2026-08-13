# tree2

Ports of [`$mol_tree2`](https://github.com/hyoo-ru/mam_mol/tree/master/tree2) —
the **tree** format's parser, serializer and AST — to Rust, Go and Python.

`tree` is a whitespace-significant structural format with three lexical
ingredients (tab, space, backslash) and no escaping rules. It is what `$mol`
uses for `view.tree`, `meta.tree` and friends, and it works just as well as a
config or IR format on its own.

```
server
	host \0.0.0.0
	port \8080
	greeting
		\Hello,
		\world!
```

## Status

| | parse | serialize | select/filter | insert/update | json | package |
|---|---|---|---|---|---|---|
| [TypeScript](https://github.com/hyoo-ru/mam_mol/tree/master/tree2) | ✅ | ✅ | ✅ | ✅ | ✅ | upstream reference |
| [D](https://github.com/nin-jin/tree.d) | ✅ | ✅ | — | — | — | upstream, separate lineage |
| [Rust](./rust) | ✅ | ✅ | ✅ | ✅ | ✅ | not published yet |
| [Go](./go) | ✅ | ✅ | ✅ | ✅ | ✅ | not published yet |
| [Python](./python) | ✅ | ✅ | ✅ | ✅ | ✅ | not published yet |

All three pass the full corpus. They were also differentially tested against
each other and against the reference over 618 adversarial inputs held out of the
corpus: the three ports agree with each other on every one, and the 63 inputs
where they part company with the reference are all the same documented defect —
see [Known reference bugs](./SPEC.md#known-reference-bugs).

## Layout

```
SPEC.md      language-neutral specification — the contract every port meets
fixtures/    shared conformance corpus, generated from the reference
reference/   verbatim copies of the $mol TypeScript sources, for consultation
tools/       builds the reference and regenerates the corpus
rust/ go/ python/
```

## The contract

Read [`SPEC.md`](./SPEC.md) first. It pins the grammar, the parsing algorithm
character by character, the serializer, the API surface, the JSON mapping, the
exact wording and coordinates of every syntax error, and the two places where
the reference implementation is wrong and the ports deliberately are not.

Every port's test suite reads [`fixtures/`](./fixtures) directly from the repo
root — no vendored copies, so a corpus change moves all three at once.

## Regenerating the corpus

The fixtures are derived from the reference implementation, never hand-written
(except `reference_bugs.json`, which is hand-written by definition).

```sh
npm i -D typescript
node tools/build-ref.js      # transpile reference/ into tools/ref.js
node tools/gen-fixtures.js   # emit fixtures/*.json
```

To add a case, edit `tools/gen-fixtures.js` and regenerate.

## Licence

MIT, following upstream `$mol`. See [LICENSE](./LICENSE).
