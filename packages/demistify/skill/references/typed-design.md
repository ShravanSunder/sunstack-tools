# Track design records across a site

Use this optional workflow when agents need to refer to requirements, rules, decisions, and planned checks by ID. Write the HTML by hand with the components that answer the reader's questions. Give each record a real HTML ID so a handoff can link straight to the explanation.

```sh
demistify design init docs/specs/main/deliverables/jobs
demistify design check docs/specs/main/deliverables/jobs --site-root docs/specs
```

Use the CLI from the selected Demistify binding. With a local npm installation, prefix each invocation with `npm exec --`. With the published archive, use the exact `pnpm dlx` URL from the matching entry guide and append the same `design` arguments.

1. `init` writes `design.ts` and its local `design-schema.d.ts` declaration. It takes Pages, Map, and Walkthroughs from an existing `navigation.json`. For a new set, use the seeded page list and author the HTML pages yourself. Keep `design.ts` a literal data object. Its type import resolves to the local declaration.
2. Add records with a stable ID, a title, a destination on a page in the set, and source links. Rules cite the needs they serve. Realizations and planned checks cite the rules they address.
3. Use `<ex-nav src="./navigation.json">` on owned pages, with the relative path adjusted for nested pages. Page links and source links may leave the set when their destinations stay inside the explicit site root. For example, `../../overview.html#scope` can link from the jobs set to the main overview. Preview a shared navigation site over HTTP.
4. Run `check` to validate the data, local targets, and Demistify markup. The outputs are `design.json` for record lookup and `navigation.json` for the browser. Edit `design.ts` to change those outputs.

An HTML fragment check verifies that the authored ID exists. Source excerpts and whether a planned check actually proves its rule still need engineering review.

If a check is forcibly interrupted while writing, remove the leftover `.demistify-design.lock` only after the checking process has ended, then rerun the check to regenerate both outputs.
