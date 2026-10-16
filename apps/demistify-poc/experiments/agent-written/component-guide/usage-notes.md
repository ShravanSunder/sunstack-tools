# Authoring notes

An explainer is HTML composition. The shared library owns rendering, typography, tones, icons and inspection.

## Markdown files

Use `ex-markdown src="./notes.md"` while serving HTTP. The source panel shows the same original Markdown. Mermaid fences render diagrams.

For a file you open directly, run `demistify embed input.html output.html` using the matching source CLI. Keep output beside input or preserve relative page/image resources when moving it. This embeds the Markdown; an offline artifact additionally bundles the library and its resources.

## Navigation

Use named presentations for complete explainers, Pages for a collection's files, and real headings for the page outline. Deep links reveal the containing presentation and tab.

## Visual inspection

Cmd+scroll zooms and Cmd+drag pans pictures and diagrams in inspection. Ordinary scrolling stays native. Buttons and Reset remain available.
