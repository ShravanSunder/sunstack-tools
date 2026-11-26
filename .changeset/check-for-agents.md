---
'@sunstack/demistify': minor
---

`demistify check` is easier for agents to use:

- `--json` prints one object: error and warning counts, each typed tree's error, warnings and report, published copies, each page's findings and any render findings.
- The summary now counts typed-tree warnings (order warnings and unregistered pages). Before, it reported "0 warnings" even when it printed them.
- A record that still holds a `TODO:` placeholder, such as one `initiative add` scaffolded, is a warning instead of passing silently.
- A link that leaves the site root names the `--site-root` folder that would hold both files.
- A page that only mentions a generated file name in its text is no longer taken for a published copy of a typed design.
