# What to reuse from The Layer Tree

**Recommendation:** reuse its reading layout, typography, decision callouts, rationale blocks, contributor notes, status links and architecture bands as shared presentation primitives. Its strongest quality is that visual treatment follows the reader's job: find the decision, understand the structure, compare the options, then inspect the details.

This is source-backed visual research for the main presentation-experiments thread. The explanations of why the patterns work are design judgments, not measured comprehension results. Component mappings below are recommendations; this report does not implement or change the five prototypes.

## Source and evidence

| Evidence                                                                                                                                                      | What it establishes                                                                                                           |
| ------------------------------------------------------------------------------------------------------------------------------------------------------------- | ----------------------------------------------------------------------------------------------------------------------------- |
| Private reference artifact, inspected in Chrome on 2026-10-14                                                                                                 | The rendered design and native anchor navigation                                                                              |
| [Extracted authored stylesheet](../../../../apps/demistify-poc/references/layer-tree-reference.css)                                                           | Exact layout, typography, theme and component rules; 9,371 UTF-8 bytes                                                        |
| Live stylesheet hash compared with the saved CSS                                                                                                              | Both have SHA-256 `ff4d4056c1f2c0118928e9c3f70115160ea9d9b2aed3cecee737570575c10524`                                          |
| Content-frame DOM inspection                                                                                                                                  | Ordinary HTML: 76 headings, 15 tables and 41 articles; no custom elements or SVG diagrams                                     |
| [Existing island runtime](../../../../packages/demistify/src/runtime.ts) and [adapted component CSS](../../../../packages/demistify/src/styles/reference.css) | Some visual primitives already have implementation homes in this lab; this is source evidence, not fresh runtime verification |

The authored document runs inside Claude's artifact viewer. The viewer header, account controls, sharing and comments belong to the host. The stylesheet studied here belongs to the content document. This is evidence about one artifact, not Anthropic's universal design system or its private prompting process.

Original screenshot omitted from the public repository: it includes host account UI. The extracted CSS remains the source evidence.

## What is worth taking first

| Pattern                                            | Reader benefit                                                             | Reuse priority | Cost or constraint                                                                      |
| -------------------------------------------------- | -------------------------------------------------------------------------- | -------------- | --------------------------------------------------------------------------------------- |
| Decision callout with question and recorded answer | Makes the governing choice easy to find                                    | First          | Loses emphasis if every paragraph becomes a callout                                     |
| Typography, reading width and spacing              | Makes dense technical prose comfortable to scan                            | First          | Fonts add delivery assets; fallback fonts must remain usable                            |
| Sticky contents and stable anchors                 | Lets readers move between overview and detail                              | First          | Needs reliable IDs and headings; sticky navigation becomes normal flow on small screens |
| Rationale and contributor notes                    | Separates the main statement, its reason and someone else's interpretation | First          | Visual difference must preserve explicit labels and evidence                            |
| Status links and legend                            | Makes an overview actionable without hiding meaning in color               | First          | Status words need a stable meaning in our domain                                        |
| Architecture bands                                 | Shows layers and their responsibilities without diagram-engine overhead    | Next           | Suitable for hierarchy; insufficient for a graph with complex edges                     |
| Definition-list detail sections                    | Gives repeated items a predictable inspection structure                    | Next           | Use only fields that help the reader; avoid empty template slots                        |
| Quiet comparison tables                            | Compares gains and costs across the same dimensions                        | Next           | Wide tables need horizontal overflow handling                                           |
| Responsive grids and text-flow blocks              | Fits parallel roles or exact textual relationships                         | Selectively    | A grid implies peers; a text diagram needs preserved whitespace                         |

## Callouts: preserve their different jobs

### Decision callout

**Source:** CSS lines 72–76, selectors `.decision`, `.decision .q` and `.decision .decided`.

The outer block uses a raised document surface, a fine border, a 4px accent edge, 20px vertical / 24px horizontal padding and a 6px corner radius. The question gets a condensed heading face at 24px with a tight 1.25 line height. The answer is a distinct inset surface inside the same block.

The valuable pattern is the relationship between the parts: an explicit decision state, the question, the recorded answer, its interpretation, and the surrounding rationale or comparison. The question remains visible while the reader assesses the answer. An accepted choice can stay where the original question was raised rather than disappearing into a separate history page.

**Reuse:** let `ex-decision` contain a heading, answer, rationale and comparison. Keep those content parts composable. Shared styles should own the geometry; the author supplies the actual decision state and wording. A visually accepted state must never create approval that the content does not establish.

Original screenshot omitted from the public repository: it includes private conversation text. The reusable geometry is documented below.

### Rationale block

**Source:** CSS lines 113–117, selectors `.lessons`, `.lesson` and `.why`.

The rationale uses a soft accent background, 12px / 16px padding, a 6px radius and a small heading label. It sits after the evidence or explanation it interprets. This produces a useful two-step reading path: inspect the claim, then understand its consequence.

**Reuse:** an `ex-callout` can carry rationale beneath Markdown, a diagram or a comparison. Prefer a semantic author choice such as rationale or risk over a raw background color. Use the block when a consequence deserves emphasis; ordinary explanation remains ordinary prose.

### Contributor note

**Source:** CSS lines 137–140, selectors `.note`, `.note.codex` and `.note.advisor`.

Notes have a compact inset treatment and a visible contributor label. Green identifies one contributor and teal identifies another. That makes commentary recognizable while preserving the main document's reading order.

**Reuse:** retain the named contributor or role in text, with a quiet theme variant. A nested note can be composed inside a decision or detail article. Contributor identity and correctness are separate meanings: a green contributor note is not proof that its contents passed verification.

### Repeated decisions

**Source:** CSS lines 143–145, selectors `.decisions`, `.decisions li` and `.decisions li.first`.

A list of decisions receives simple bordered surfaces. The leading decision has a stronger accent edge. This distinguishes an outstanding choice that affects later choices without inventing a separate visual language for every item.

**Reuse:** use an ordinary list inside the decision or Markdown island. Add emphasis only when a dependency or priority warrants it. This pattern does not require another custom element.

## Typography and layout details

**Source:** CSS lines 3–70.

| Role                       | Source choice                                    | Why it is useful                                                          |
| -------------------------- | ------------------------------------------------ | ------------------------------------------------------------------------- |
| Headings and navigation    | IBM Plex Sans Condensed                          | Dense labels and headings remain compact and visually distinct from prose |
| Body                       | Literata, 17px, line height 1.6                  | Gives a long explanation a reading-document character                     |
| Code and exact identifiers | IBM Plex Mono                                    | Keeps copyable names and snippets recognizable                            |
| Main title                 | 44px, weight 700                                 | Establishes the document topic without an oversized hero section          |
| Section heading            | 30px, weight 600, 64px top spacing               | Separates substantial sections with whitespace                            |
| Subsection heading         | 22px, weight 600, 36px top spacing               | Provides a second clear level of hierarchy                                |
| Paragraphs and list items  | Maximum width 68ch                               | Limits long lines while diagrams and tables can use a wider surface       |
| Lead paragraph             | 20px, line height 1.55, maximum width 62ch       | Gives the opening explanation more prominence                             |
| Page frame                 | Maximum width 1180px; 220px navigation; 48px gap | Gives navigation a stable home without consuming the reading column       |

The page uses CSS Grid with a flexible reading column and explicit minimum-width handling. The body has 20px horizontal padding. At 900px the two-column frame becomes a single column, and the navigation stops being sticky. At 640px, architecture bands and definition-list fields stack vertically.

**Adaptation:** retain the three typography roles, line lengths and spacing relationships. Treat exact pixel values as a coherent starting point. Package fonts locally when an artifact must work without remote font loading; preserve the source's fallback stacks. CSS size does not include the font files.

## Architecture bands and detail articles

### Layer bands

**Source:** CSS lines 89–105, selectors `.tree`, `.band`, `.band .lv`, `.band .body`, `.band.cross` and `.band.proposed`.

The overview is a stack of bands with a 10px gap. Each band contains a 150px label column and a flexible content column. The label area carries the layer name and kind; the content area carries the responsibility question and links to individual items. Corners are 8px. A proposed band uses a dashed border; a cross-cutting band uses a quieter label treatment.

**Why take it:** this is an inexpensive architecture view whose layout directly expresses grouping. The colored label column provides a repeated landmark, and the links make it an entry point into the detailed document.

**Adaptation:** `ex-layer` can own the shared band styling, with ordinary HTML children for the label and body. Keep proposed or cross-cutting meaning explicit in text as well as border treatment. Use Mermaid or D2 when relationship edges are essential; a band layout alone cannot represent them.

### Structured detail articles

**Source:** CSS lines 122–140, selectors `.skill`, `.skill header`, `.skill dl`, `.skill dt` and `.skill dd`.

An article header combines its identifier and status. Its body uses a definition list with a 90px field-label column, a flexible explanation column, and 6px / 14px gaps. Labels are small and muted; the content remains readable prose. Articles use `scroll-margin-top: 24px` so anchor destinations have breathing room.

**Why take it:** repeated items are easier to compare when fields appear in consistent positions. A reader can inspect purpose, boundary, rationale and evidence without learning a different layout for every item.

**Adaptation:** reuse ordinary `article`, `header`, `dl`, `dt` and `dd` markup under shared styles. An extra custom element is worthwhile only if it adds behavior or meaning that composition cannot express. Fields should follow the material being explained, rather than forcing every item to fill a checklist.

## Status colors: reuse the mechanism, preserve the meaning

**Source:** CSS lines 3–44 and 78–87.

The stylesheet pairs each semantic foreground with a soft background, then changes those pairs together in dark mode. Chips contain both a dot and a word. Missing items also have a dashed border.

| Artifact meaning | Treatment                                   |
| ---------------- | ------------------------------------------- |
| Keep             | Neutral gray                                |
| Change           | Amber                                       |
| Strengthen       | Blue                                        |
| Missing          | Red with dashed outline                     |
| Being patched    | Purple                                      |
| Contributor note | A separate green or teal identity treatment |

The important reuse is centralized semantic styling. These artifact meanings are not automatically the same as graph-diff meanings. For an architecture diff, keep additions green, removals red, modifications amber, unchanged elements neutral and focus in the theme accent. Contributor identity, evidence confidence, workflow status and graph change must not share an ambiguous color meaning.

Authors should declare a meaning; the theme chooses its visual representation. Preserve a textual label so the meaning survives grayscale, different themes and color-vision differences. The source's light and dark palettes are implemented; their contrast ratios were not audited in this study.

## Interaction patterns actually present

| Pattern                           | Evidence                                                                         | Useful transfer                                                                                |
| --------------------------------- | -------------------------------------------------------------------------------- | ---------------------------------------------------------------------------------------------- |
| Contents link to a section        | Native anchors; clicking changed the content-frame hash to the selected section  | Stable, shareable navigation with no application router                                        |
| Overview item to detail           | Chips are links to article IDs; native anchor structure inspected                | A small overview can remain connected to a large explanation                                   |
| Sticky reading navigation         | CSS lines 49–54; source-inspected                                                | Keep the document's map available while reading                                                |
| Hover and keyboard-focus feedback | Border/color rules for contents and chip links; global focus outline at line 158 | Show which target is actionable and where keyboard focus is                                    |
| Smooth anchor scrolling           | CSS line 160                                                                     | Connect the overview and destination visually                                                  |
| Reduced-motion handling           | CSS line 159                                                                     | The source declares a reduced-motion override; verify its final computed behavior before reuse |
| Table and text-flow overflow      | `.tbl` and `pre.flow` use horizontal overflow                                    | Preserve wide technical information without shrinking it into unreadability                    |
| System-aware theme                | Media query plus explicit root theme attributes                                  | One set of component geometry works across light and dark palettes                             |

**Observed scope:** contents navigation was driven in Chrome and the resulting section view was inspected. Item-to-detail relationships were inspected in the DOM. Focus, overflow, responsive layouts and reduced-motion behavior were read from CSS; they were not individually exercised. The reduced-motion rule uses a universal selector, which has lower specificity than the root's `html` smooth-scroll declaration. Adapt it with a root-specific override and verify computed behavior under a reduced-motion preference before adoption.

The content DOM contained no buttons. This study found no source evidence for document tabs, expand/collapse widgets, diagram zoom, pan, fullscreen, status filtering, automatic active-section tracking or saved theme preferences. Those may be useful additions to our library, but they are not interactions demonstrated by this artifact. The host's sharing and comment controls are separate.

## Tables, parallel content and exact text

**Source:** CSS lines 107–111 and 146–157.

Tables use restrained horizontal rules, left alignment, top-aligned cells and compact headings. They organize choices without competing with the decision callout. The surrounding overflow wrapper keeps wide comparisons available on narrow surfaces.

Responsive grids use content-sensitive minimum widths rather than a fixed number of columns. This is useful for genuinely parallel roles or options. It should not turn an ordered explanation into an arbitrary card grid.

The text-flow block preserves whitespace in a monospaced surface with horizontal scrolling. It is useful for exact relationships or a small terminal-oriented sketch. It is ordinary preformatted text, not an interactive diagram renderer.

## How to carry this into HTML Islands

| Existing source pattern                 | Reuse home                                             | Author supplies                                              |
| --------------------------------------- | ------------------------------------------------------ | ------------------------------------------------------------ |
| Reading frame, fonts, theme and spacing | Shared document stylesheet                             | Document title, headings and content                         |
| Decision geometry                       | `ex-decision`                                          | Question, state, answer, rationale and any nested comparison |
| Rationale and note treatment            | `ex-callout` or ordinary labeled note inside an island | Meaning, attribution and prose                               |
| Band geometry                           | `ex-layer`                                             | Layer name, responsibility and nested content                |
| Structured articles and comparisons     | Ordinary HTML or Markdown inside islands               | The relevant fields and comparable facts                     |
| Navigation and anchor affordances       | Shared viewer/document behavior                        | Stable IDs and link targets                                  |
| Status treatments                       | Shared semantic theme                                  | Status meanings rather than raw colors                       |

The inspected lab runtime already applies the decision, rationale and band classes to `ex-decision`, `ex-callout` and `ex-layer`. The shared component stylesheet contains their geometry. That is a useful existing home; inspect and extend it rather than introducing a second style system. This report does not establish complete field rendering, accessibility or runtime parity for those components.

Compose a callout inside a comparison, a note inside a decision, or a Markdown explanation inside a layer. The reusable library owns styles and interactions. The agent's document owns content and composition. Library implementation, theme CSS and font assets stay outside per-explainer authoring-token counts; asset sizes can be reported separately.

## A repeatable way to learn from good artifacts

Use each admired artifact to grow a small evidence-backed pattern catalog. This is a proposed working method, not an implemented collector or an ongoing browser task.

1. Name the reader's job that the pattern solves: make a choice, orient, compare, inspect detail or follow evidence.
2. Capture the authored content separately from the viewer UI, with source URL, date, screenshot and a content/style hash.
3. Record the smallest pattern: its structure, selectors, design tokens, interaction, responsive behavior and reason to use it.
4. Mark source-inspected behavior separately from interactions actually driven. Keep inferred design rationale labeled as interpretation.
5. Adapt the meaning to our domain. Give status, attribution and confidence their own explicit labels.
6. Fit the pattern into existing primitives. Add a component only when composition cannot carry its meaning or behavior.
7. Demonstrate the adapted pattern with long text, long identifiers, narrow layouts, dark mode, keyboard focus and anchor navigation. Only then label it verified in our library.

Preserve source attribution with extracted assets. Retain the visual ideas and component geometry; account controls and host frame scripts belong to the surrounding viewer.

## Main-thread handoff

Use this report as research input for the shared presentation layer. The immediate candidates are decision callouts, rationale/contributor notes, reading typography, contents navigation, status links and layer bands. Their source selectors and values are above; the saved stylesheet matches the live artifact exactly, with private screenshots retained outside this repository.

Carry the patterns into the existing HTML Islands runtime and shared CSS where they fit. Keep additional pattern demonstrations outside the equal-content five-prototype comparison. Separate per-document usage from reusable library code when reporting tokens. Preserve the original meaning of decisions, attribution and graph changes while adapting their presentation.

Remaining limits: one artifact studied; original prompts and production design instructions unknown; no estimate of the effort spent making it; no controlled comprehension or authoring-cost study; no mobile, keyboard or contrast audit; no adopted-library runtime proof from this side conversation.
