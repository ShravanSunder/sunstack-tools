# Prepare a double-click reader

Browsers block sibling-file fetches from `file:` pages. Embed local Markdown while authoring, retaining the HTTPS library links:

<!-- Release staging binds the logical archive filename below to the built archive's content hash. -->

```sh
pnpm dlx https://demistify.getagentstudio.dev/v1/demistify-cli.tgz embed /path/index.source.html /path/index.html
pnpm dlx https://demistify.getagentstudio.dev/v1/demistify-cli.tgz check /path/index.html
```

Run with Node 26+. An installation of the CLI archive also resolves its declared TypeScript dependency. Install or cache that dependency before offline use. Keep output beside input or preserve relative page/image paths. The transform safely carries exact source, including a script terminator; it does not inline the runtime, fonts, pictures or arbitrary JSON. The reader runs no command. Later source edits require another embed.

After editing a known live source, regenerate its reader and collection index when present. Routine delivery needs no second temporary copy or byte-comparison pass. Use a comparison only to investigate a stale-output defect or an explicit freshness-proof request.

Ordinary relative HTML links, fragments and picture paths work when their files travel together. Inline supplied Explorer JSON or use an allowed HTTPS data URL for direct-file delivery; embedding Markdown does not prepare Explorer data. HTTPS assets need a network. Truly offline delivery needs the complete matching local distribution or the source API's standalone bundle, not just `runtime.js` and `theme.css`; [Runtime trust for sensitive content](runtime-trust.md) says when that bundle counts as checked.
