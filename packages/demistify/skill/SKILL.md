---
name: demistify-explainer
description: "Use when writing, editing, or checking Demistify markup: component syntax, attributes, nesting, layouts, Markdown/Mermaid islands, and data references."
---

# Demistify explainer

Author a document, not another application. You own its explanation, source facts and ordinary HTML layout. Demistify supplies reusable views, controls, rendering and shared styles. Keep prose in document flow; choose an island when it gives the reader a useful view or action. Layout CSS is allowed; use the library's typography, colors, borders, radii and shadows.

Pages are separate HTML documents. Tabs switch peer views in one place. A native disclosure reveals optional depth locally. A Walkthrough orders stops through existing content and may cross pages; a Gallery Journey is a picture-only path through existing figures. Neither copies the content nor replaces ordinary navigation.

## 1. Bind the page and prepare its sources

Use the supplied library base, normally `https://demistify.getagentstudio.dev/v1/`. MUST load `release.json` and `authoring/SKILL.md` relative to that base and return the base, version/source commit and confirmation that theme, runtime and chosen API belong to it. This exact matching guide read counts; do not reload it recursively. Experimental v1 can change. A source preview uses its matching local build and names absent release metadata instead of claiming a published binding.

```html
<!doctype html>
<html lang="en">
<head>
  <meta charset="utf-8">
  <meta name="viewport" content="width=device-width,initial-scale=1">
  <title>Session ownership</title>
  <link rel="stylesheet" href="https://demistify.getagentstudio.dev/v1/theme.css">
  <script type="module" src="https://demistify.getagentstudio.dev/v1/runtime.js"></script>
</head>
<body>
  <ex-nav for="content"></ex-nav>
  <ex-document id="content">
    <h1>Session ownership</h1>
    <p>The Registry retains the session; a connection carries messages.</p>
  </ex-document>
</body>
</html>
```

Ordinary `ex-document` pages get Comments automatically, initially closed, with a page-address identity, a content fingerprint and targets for semantic content. Author no feedback tag, page name/version or paragraph labels for this default. Use `comments="off"` only for a deliberate opt-out; choose an explicit identity or revision only when its namespace consequence is intended.

MUST load `references/content.md` and return the actual source carriers, image/data references and intended delivery method. A reader can double-click a prepared HTML file: HTTPS library assets need the network, not a local application/server. Sibling Markdown/JSON fetches need HTTP or author-time embedding. Keep relative page/image paths intact. Offline delivery also needs the complete matching library; two copied asset files are insufficient.

Completion: the binding and opening method are concrete, supplied sources are preserved, and every required fetch can work through that method.

## 2. Choose the views and load their detail

Two widths share one left edge from 1.10.0. **Reading** holds prose at 760px; **Canvas** gives visuals up to 1600px. Text remains left aligned. Document headings and dividers follow Reading; a wide view and its controls stay together. Earlier bindings follow their own guide. Set `data-width="reading"` or `data-width="canvas"` on the whole component or native layout wrapper when choosing explicitly.

For each matching condition below, load the named reference and return its authored result. The references own syntax, configuration, controls, composition and deciding observations; a list of component names is not the result.

| IF the page needs… | Load | Return |
| --- | --- | --- |
| Explicit widths, a grid or tabs | `references/layouts.md` | Whole-component widths, child layout, labels and narrow behavior |
| A card, consequential callout or reported number | `references/cards-and-callouts.md` | Meaningful units, tones/icons and truthful recommendation/chosen states |
| The reader must select answers, attach feedback or override/disable the default review | `references/choices-and-comments.md` | Requested response/feedback arrangement, default or overridden identities and binding, optional supporting details and interactions to observe |
| A compact status or identity, exact progress count or named phase comparison | `references/small-parts.md` | Supported parts, the complete combined-row recipe when requested, truthful boundaries, stable phase slots and a narrow/theme observation |
| Pages, heading links, breadcrumbs or collection search | `references/navigation.md` | Native page links, stable targets and current collection index when needed |
| Sidebar/fullscreen inspection, including a visual's default controls | `references/inspection.md` | Inspection choice and return/state behavior to observe |
| A Mermaid view or Mermaid fence | `references/diagrams.md` | Supported family, safe source and readable, resolving targets |
| Code, a diff or line notes | `references/code.md` | Exact source and notes on displayed file/row numbers |
| Ownership regions, routed arrows or optional layers | `references/maps.md` | Reading/map choice, real endpoints and room for complete captions |
| A picture, screenshot collection or image slider | `references/pictures.md` | Ordinary figures or a dedicated Gallery page with actual capture metadata |
| A guided path, including a Gallery Journey | `references/tours.md` | Named/described path, focused narration and resolving stops |
| Alternatives on corresponding topics | `references/comparisons.md` | Matching topic keys, authored columns/tabs and natural content height |
| Hierarchy, file paths or a literal text drawing | `references/trees.md` | Tree/File Tree/Pre choice, independent folding/jumps or exact text |
| An ordered explanation with expandable evidence | `references/traces.md` | Native step disclosures with source/observation status |
| Source-derived call-path changes | `references/call-paths.md` | Calls, branches and unknown execution kept distinct |
| A supplied Explorer graph document | `references/explorers.md` | Original graph data, real selection IDs and source-backed views |

Completion: selected constructs answer the reader's task, their references were applied, and every construct is supported by the chosen binding.

Answer controls follow the requested task: feedback alone needs no choice, and a choice needs no extras. Use the existing Comments panel for feedback and a combined handoff; no separate answer-copy strip or page-owned response store is needed.

## 3. Compose content once and keep it navigable

Use stable island IDs and meaningful document headings. Keep the small `ex-nav` from the shell on every page offering a Walkthrough or Gallery Journey; the path declaration itself is hidden. A collection adds native Pages links inside that same navigation element. Do not require starting a guided path to read a page or follow an ordinary heading link.

A card groups one named option, component or finding. A callout highlights a proposal, decision, risk, question or unverified outcome; a subject icon says what it concerns. A recommendation is not an owner's chosen option. Local optional detail uses `details`/`summary`, not a Walkthrough. Rich content may nest, but another frame must add meaning: a tabbed alternative containing a Flow rarely needs a large outer card around the Flow's own cards.

Use the generated vocabulary at the matching binding for accepted attributes and values. Do not invent elements, tone names or CSS variants from a screenshot. Gallery browsing belongs on its own page: one `ex-gallery` directly inside `ex-document`, optionally after one h1. Put navigation and hidden path declarations beside the document; an explanation uses an ordinary picture or a link there. The matching authoring checker reports `gallery-page` for embedded or multiple galleries; runtime support for older compositions does not make them valid authoring.

Completion: facts occur once, navigation resolves to the intended content, proposal/evidence labels are honest, and ordinary examples contain reader content rather than state-retention test inputs.

## 4. Check the delivered files and observe the result

The CLI ships in source, not npm. From a dependency-installed matching sunstack-tools checkout with Node 26:

```sh
node packages/demistify/bin/demistify.ts check /absolute/path/to/page.html
```

Check every final HTML file after embedding/indexing. Require a nonzero file count and **zero errors and warnings**; exit 0 alone does not establish both. `check --fix` repairs mechanical findings. The checker validates recognized markup/data, not prose truth, matching import releases, useful layout or readable diagrams. Compare theme/runtime imports with the recorded base explicitly.

Open the actual artifact through its intended supported route. Exercise its authored links, chosen controls, loaded content, narrow layout and any inspection/fullscreen return. Inspect rendered Markdown and SVG rather than assuming source text rendered. Compare embedded source with a fresh regeneration from the live source page as `content.md` teaches. Follow browser URL/access policy; a blocked file URL is a proof gap, not a reason to launch another browser or server.

Runtime authoring issues appear in console warnings and the root's `data-ex-issues` count. `?ex-check` opts into their visible panel; readers do not need that panel to see a content-load error. Fix the authored issue or report the exact release limitation.

Completion: matching static checks and source comparisons pass; the actual observations establish the claims being returned, with unavailable interactions explicitly unverified.

## 5. Return the artifact and its evidence

Return the file link, recorded library binding/release, checks and actual observations, plus remaining limitations. Count authored HTML, referenced content/data/images and shared library costs separately. Source bytes/tokens are not model input/output/thinking usage or billed dollars. Loading public assets does not publish the document; publish only within the user's authorized scope.

The task is complete when its requested content and interactions work through the intended delivery method, or it is returned with the precise blocker and no unsupported completion claim.
