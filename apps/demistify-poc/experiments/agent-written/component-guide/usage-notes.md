# Authoring notes

An explainer is HTML composition. The shared library owns rendering, typography, tones, icons and inspection.

## Markdown files

Use `ex-markdown src="./notes.md"` while serving HTTP. The source panel shows the same original Markdown. Mermaid fences render diagrams.

For direct-file viewing, use the shared Cloudflare library URLs and embed only local Markdown with `node packages/demistify/bin/demistify.ts embed input.html output.html`. Keep image and page references as normal paths/URLs. The reader opens small HTML directly; no Python/Node/Vite server is needed. HTTPS Markdown/data URLs may remain live references when their origin permits CORS. Internet access is needed for Cloudflare imports. The same files also work on an ordinary static server. This source preview uses matching local assets; reader delivery must use the matching released channel.

## Navigation

Pages lists the collection’s HTML files, with Home first. The library derives breadcrumbs and the page-named outline from the native page links and real headings. A presentation is a named, described ordered path through existing islands or inner anchors; it never owns or duplicates their content. Deep links reveal inactive tabs, layers and disclosure branches.

## Visual inspection

Cmd+scroll zooms and Cmd+drag pans pictures and diagrams in inspection. Ordinary scrolling stays native. Buttons and Reset remain available.
