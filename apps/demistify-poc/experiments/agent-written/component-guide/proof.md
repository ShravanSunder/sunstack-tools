# Delivery and checks

## Open directly

Keep the HTML small. Load shared styling and runtime from the same Cloudflare `/v1/` channel. The browser downloads fonts, renderer chunks and the worker as needed; the HTML does not contain those assets.

Double-click the HTML to read it. Relative HTML page links, fragments and image paths work without a local server. Keep referenced images beside the pages, preserving the authored folder structure. Internet access is needed for shared HTTPS assets.

## Choose how Markdown loads

| Source | Delivery |
| --- | --- |
| Inline Markdown | Render directly from the HTML |
| Local `.md` beside the HTML | Run the source CLI's `embed` transform once while authoring |
| HTTPS `.md` | Keep `src` when its origin permits cross-origin reads |

The embed transform retains the exact Markdown and external library links. It does not bundle JavaScript, fonts or images. Rerun it after changing the source file; preserve the relative image and page paths when placing the output.

Any ordinary static HTTP server can optionally host the same files. Demistify requires no special server, framework or backend. Serving local Markdown over HTTP allows live `src` reads during editing.

## Verify what readers see

Run the matching source checker, then inspect the actual browser result. Confirm tables, code, task lists, images and Mermaid render; open Markdown source and compare it with the original. Follow a cold heading/node link and every presentation stop. In galleries, filter, switch review/grid, use arrow keys and open the caption beside the main view.

A checker pass does not prove browser interactions. An HTTP pass does not establish direct-file operation. A local build does not establish that Cloudflare serves the same release: inspect `/v1/release.json` and use its matching authoring guide.
