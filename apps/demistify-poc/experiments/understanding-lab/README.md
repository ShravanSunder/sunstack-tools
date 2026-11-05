# Understanding lab

The Requirements and source-derived call-path experiments use the local Demistify build, native Web Components and shared theme. Linked-record authoring and document projections remain private to the demonstration app. Shared bottom-drawer behavior, rendering, Comments and styles belong to the library. Completed corner/category-color studies are retained as source archives.

Build from the repository root:

```sh
pnpm --filter @sunstack/demistify build
node apps/demistify-poc/scripts/build-understanding-lab.ts
pnpm --filter @sunstack/demistify-poc dev:serve --port 4196
```

Open `http://127.0.0.1:4196/experiments/understanding-lab/index.html`.

| Page                | Current direction                                                            | Still available to compare                                                          |
| ------------------- | ---------------------------------------------------------------------------- | ----------------------------------------------------------------------------------- |
| `requirements.html` | Capability groups, searchable picker, neutral cards and a quiet detail panel | Previous/Next, responsive list/detail, scoped keys, exact need and stage addresses  |
| `execution.html`    | Searchable stack selection, nested calls and right-hand details              | Three illustrative stacks, supporting-call disclosure and hover-only detail preview |

Completed studies: [`corners.html`](corners.html) and [`color.html`](color.html). Their files remain inspectable, but they are no longer active index or page-menu entries. The selected need offers equal-size Overview, Specification, Program Design and Verification controls. The separate related-documents action opens a non-modal bottom workspace: `requirements.html?need=U2&reading=documents` compares documents together, while `reading=rule-path` reads a rule and its related design/checks in order. Both use the same authored records. The call-stack action opens the same workspace and returns to the originating view. Independent style/component integration review remains pending.

The Gateway/Registry/Session records, requirements, source addresses and observations are authored illustrative fixtures. They are not an assessment of a shipped product or a measured runtime trace. A green example does not prove a product requirement. Supporting calls without a direct requirement mapping never borrow another call's result.

## Focus and navigation

Both browsers use the shared Canvas width, a quiet 8 px focus outline with interior padding, and the same private `lab-search-picker`. Its caller supplies labels, option details and selection meaning. It owns the search icon, native input, bounded result list, autofocus, arrow selection, Enter and Escape. Whole-document search retains its separate ownership and matching behavior.

- Requirements: in wide views or the narrow detail view, Previous/Next and Left/Right arrows move through the currently filtered needs. A one-need filter disables both buttons. The picker and editable controls keep their own keys.
- Wide requirements views use a list column clamped to 300–520 px at 35% of the available width, beside a padded detail surface. At 740 px or less of component width, the list and detail become separate views. A tap opens full-width detail; Back restores the filter and selected row. Browser Back/Forward work too. Reading uses normal page scrolling, and Previous/Next sit after the detail; phone keyboard hints are hidden.
- Call stacks: Left/Right choose a stack; Up/Down select visible calls. These keys apply only while the component has focus. Hidden plumbing stays behind an explicit disclosure. No Next call button or alternate flat view remains.
- Hover previews only the call detail pane. It never changes the pinned selection, evidence chain, URL, opacity or visibility of other calls. Click or keyboard activation pins a call.
- A picker selection returns focus to its trigger. Returning to All capabilities preserves the selected need if it remains visible.

## Links agents can use

IDs are authored in inline inert JSON in their owning HTML pages. Requirements owns U1–U6; Specification owns S1–S6; Program Design owns C1–C5; Verification owns scenarios and separate observations; execution owns frames and stacks. Rich body templates are referenced by stable IDs. Agents use those IDs, rather than deriving them from display text or generated comment anchors. These address forms are an agent-facing contract; the page has no sharing or Copy link controls:

| Target                       | Relative address                                               |
| ---------------------------- | -------------------------------------------------------------- |
| User need                    | `requirements.html?need=U4`                                    |
| Its specification            | `specification.html?need=U4#specification-U4`                  |
| Its program design           | `program-design.html?need=U4#program-design-U4`                |
| Its verification             | `verification.html?need=U4#verification-U4`                    |
| A linked stack for that need | `execution.html?need=U4`                                       |
| Whole stack                  | `execution.html?stack=choose-machine`                          |
| Exact call                   | `execution.html?stack=choose-machine&call=select-list-commit`  |
| Hidden supporting call       | `execution.html?stack=start-session&call=event-bridge-forward` |

The need-to-stack entry chooses the authored primary stack when available; an unmapped need such as U5 retains its own evidence and an explicit gap. An exact-call link expands its supporting-call group when necessary. Agents combine these relative addresses with the host serving the document. Contextual navigation between a requirement and its call path remains part of the reading experience.

The illustrative stacks are `start-session`, `open-help` and `choose-machine`. The same function can occur in several stacks, so an exact call address includes both stack and call IDs.

## Source and limits

Implementation lives in `src/understanding-lab/`; pages and comparison styles live here. The existing build script validates all inline records, body IDs and forward references before writing, embeds one generated collection snapshot into each output page, and bundles the private components with Vite. Browser views never fetch sibling data or fall back to old facts. There is no framework, backend or persisted response store.

Production radius tokens remain 4/8/12 px. Mermaid palette slots use encountered identities unless categories are declared explicitly; colors do not infer domain types. The prototype's categories are explicit. Ordinary card surfaces use 8 px; stage rails are separators inside the detail surface. Card comparisons keep their base color, content, dimensions and shadow equal; only the named bands or body change. In the light theme, the base is already white, so the lighter-body variant reaches the same white limit.

Native tables use the shared library surface and retain their existing comment-target ancestry. These local examples and illustrative results do not establish deployed-product behavior or release acceptance.

Forward relationships are authored once: specification → need, design/scenario → specification, observation → scenario, stack → frames and primary need, frame → optional need. Backlinks and requirement summaries are generated. C1 serves both U1/U2 through S1/S2; U2 has an illustrative failed pointer scenario and a separate unrun keyboard scenario. U5/U6 remain Not run. There is no aggregate Pass rule. Collection-local IDs and ordinary page/fragment addresses do not promise record move semantics. The generated snapshot is not an authoring input.

## Record structure

The current fixture contains 42 records: six needs, six specification rules, five program designs, seven scenarios, four observations, eleven call frames and three stacks. The authoritative schema is [`collection-data.ts`](../../src/understanding-lab/collection-data.ts).

Metadata names identities and forward references. Rich explanation stays in the owning HTML page, with `bodyRef` and `summaryRef` pointing at its body and summary. The preparation step validates every owner, body reference and edge kind before generating a collection with `record`, `page`, `summary` and `bodyHtml`. Call frames and stacks carry their supplied inline fields.

```json
{
	"kind": "program-design",
	"id": "C1",
	"title": "Input scope routing",
	"specifications": ["S1", "S2"],
	"bodyRef": "body-C1",
	"summaryRef": "summary-C1"
}
```

C1 is shared rather than copied for U1 and U2. V2 and V2-keyboard are separate scenarios; an observation attaches to its scenario. Missing observation means Not run, rather than a stored passing result. The preparation script and schema are experimental app inputs, not a universal public graph API.
