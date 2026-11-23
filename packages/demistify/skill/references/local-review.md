# Open a local review

Use the installed CLI or the exact archive URL from the matching guide. This serves the authored page and its local inputs on loopback and opens the system browser.

<!-- Release staging binds the logical archive filename below to the built archive's content hash. -->

```sh
pnpm dlx https://demistify.getagentstudio.dev/v1/demistify-cli.tgz open docs/review/index.html
# Or: npx --yes --package=https://demistify.getagentstudio.dev/v1/demistify-cli.tgz demistify open docs/review/index.html
```

The page's directory is the default site root. Use `--site-root docs` when a collection references pages or assets elsewhere under `docs`. Use `--no-browser` in an agent's background process and give the reader the printed address. Keep the process running while the reader uses the link. Reload to see source edits. The preview opens the same page with its Markdown, JSON and relative links intact. Use Node 26+ for the Node invocation. With Bun, run the installed CLI entry point as `bun ./node_modules/@sunstack/demistify/demistify.js open docs/review/index.html`, or use `bunx --bun --package "$archive" demistify open docs/review/index.html` with the archive URL from the matching hosted guide.
