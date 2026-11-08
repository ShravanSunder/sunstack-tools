# Prepared document browsers

Keep the supplied facts in owner HTML and inline metadata. The public readers derive coordinated views from that source: `ex-requirements` offers a need list, Overview, Specification, Program Design, Verification and related documents; `ex-call-stack` offers supplied call paths. Ordinary explanation, page layout and links remain HTML.

Use the library binding selected in the main guide. This source guide documents the matching candidate API; an older published binding may not include it. A source preview uses its complete matching local distribution and CLI. Publication of a matching guide/runtime is a separate release step.

Return the owner pages, the record-to-body/part mapping, the reader markup and the actual preparation/check result. Completion means every supplied record and declared edge has its proper owner, preparation succeeds, and the final changed HTML passes the matching checker. Author behavior or browser behavior is not established by that source check.

## Put facts at their owners

The collection is one directory of sibling HTML files. Its minimum is `requirements.html` with at least one `need`. Later owner pages are optional. Each owner page that exists has exactly one inline `<script type="application/json" data-ex-records>` containing a JSON array, even when that array is empty.

| Owner filename | Record kinds |
| --- | --- |
| `requirements.html` | `need` |
| `specification.html` | `specification` |
| `program-design.html` | `program-design` |
| `verification.html` | `scenario`, `observation` |
| `execution.html` | `frame`, `stack` |

An extra `index.html`, plan or explanatory page can contain ordinary content and either public reader, but no `data-ex-records` block. Preparation includes all sibling `.html` files. Use native page links to connect them; a browser is a view within a page, not the collection's only navigation.

```html
<ex-requirements id="route-needs" label="Trail planning needs"></ex-requirements>
<ex-call-stack id="route-calls" label="Supplied route calls"></ex-call-stack>
```

Each reader needs a stable, unique `id`; `label` optionally names its control group. Defaults are “Requirements browser” and “Call stack browser”. At most one of each reader is supported in one HTML document. They take their facts from preparation output, with no `src`, inline configuration object, application script or private app stylesheet.

### Records and forward relationships

The implemented contract is owned by `src/design-documents/collection.ts`, with ownership/part checks in `src/node/design-markup.ts` and directory preparation in `src/node/design-collection.ts` of the matching library source. The following is an authoring lookup, not another schema.

All records require `kind` and `id`. Record IDs are unique across the whole collection. IDs and `bodyRef`/`summaryRef` values begin with an ASCII letter and then contain only letters, digits or hyphens. Text fields are nonempty; undeclared fields are rejected.

| Kind | Other required fields | Relationships and optional fields |
| --- | --- | --- |
| `need` | `title`, `capability`, `bodyRef`, `summaryRef` | None |
| `specification` | `title`, `needs`, `bodyRef`, `summaryRef` | `needs` is a nonempty array of need IDs |
| `program-design` | `title`, `specifications`, `bodyRef`, `summaryRef` | `specifications` is a nonempty array of specification IDs |
| `scenario` | `title`, `specifications`, `bodyRef`, `summaryRef` | `specifications` is a nonempty array of specification IDs |
| `observation` | `scenario`, `outcome`, `bodyRef`, `summaryRef` | `scenario` is one scenario ID; `outcome` is `Pass`, `Fail` or `Partial` |
| `frame` | `name`, `source`, `responsibility`, `statement`, `category` | `requirement`: one need ID; `supporting`: boolean; `branch`: nonempty text |
| `stack` | `title`, `description`, `primaryNeed`, `frames` | `primaryNeed` is one need ID; `frames` is a nonempty ordered array of frame IDs |

Frame `category` is exactly `Input`, `Resolution`, `Action` or `Plumbing`. Frames and stacks have no body/summary references. `source` and `statement` describe the supplied source path; they are not a measured execution trace. A frame's optional `requirement` points to a need, not to a specification. Backlinks and need-to-design/scenario associations are derived from these forward relationships; write the relationship once.

For the other five kinds, `bodyRef` identifies a same-page HTML container, often a `template`. Its **contents** become the record body. `summaryRef` identifies a nonempty text-bearing element inside that container (the container itself is also allowed). The summary is compact text for lists; full reading retains the body HTML. One container has one record owner. Every authored HTML ID is unique within its page, including IDs inside templates; keep supplied IDs on their original elements.

Metadata carries identities and edges; prose, tables, diagrams and evidence stay in the HTML bodies. Serialize metadata as JSON and escape any literal `<` in that JSON as `\u003c` before putting it in the script carrier. Preparation derives the cross-page snapshot from those owners.

### Mark only the supplied parts

`data-ex-part` names selected fragments for the related-document view. It belongs inside an owned record body, occurs at most once per part in that body, and takes only the values allowed for its kind:

| Kind | Allowed parts | Meaning |
| --- | --- | --- |
| `specification` | `rule`, `cases` | The exact observable rule; its supplied case table or other case content |
| `program-design` | `sketch`, `gaps` | The supplied design sketch; explicit missing explanations or limits |
| `scenario` | `procedure` | Proposed steps, kept separate from supplied observations |
| `need`, `observation`, `frame`, `stack` | None | No part markers |

Parts are optional: a missing part contributes no selected fragment, while the full record remains available through Source details. Omit unwritten material rather than labeling unrelated notes as a rule or fabricating a sketch. Unknown, forbidden, duplicate or unowned markers are preparation/check errors. Declared missing or wrong-kind record links are preparation errors; an absent later stage is valid.

## Complete invented collection

This example describes a fictional trail-planning design. The observation is an explicitly illustrative supplied result, and the two source calls are a description, not execution evidence. Each HTML block below is the complete named file. Put them and `route-notes.md` in `trail-guide/`, alongside a complete matching distribution in `trail-guide/demistify/`. For another supplied binding, replace each stylesheet/runtime URL and use its matching CLI.

### `requirements.html`

```html
<!doctype html>
<html lang="en">
<head>
  <meta charset="utf-8">
  <meta name="viewport" content="width=device-width,initial-scale=1">
  <title>Trail planning needs</title>
  <link rel="stylesheet" href="./demistify/theme.css">
  <script type="module" src="./demistify/runtime.js"></script>
</head>
<body>
  <ex-nav for="needs">
    <nav slot="pages" aria-label="Pages">
      <a href="requirements.html">Requirements</a>
      <a href="specification.html">Specification</a>
      <a href="program-design.html">Program Design</a>
      <a href="verification.html">Verification</a>
      <a href="execution.html">Execution</a>
    </nav>
  </ex-nav>
  <ex-document id="needs">
    <h1>Plan an available route</h1>
    <p>The seasonal register supplies closure notices.</p>
    <ex-requirements id="trail-needs" label="Trail planning needs"></ex-requirements>
  </ex-document>
  <script type="application/json" data-ex-records>
  [{"kind":"need","id":"NeedClosure","title":"See a closure before choosing","capability":"Route planning","bodyRef":"need-body","summaryRef":"need-summary"}]
  </script>
  <template id="need-body">
    <p id="need-summary">Show the <a href="#seasonal-notice">seasonal notice</a> before I choose a route.</p>
    <details open>
      <summary>Notice wording</summary>
      <p id="seasonal-notice">The north footbridge is closed through May.</p>
    </details>
  </template>
</body>
</html>
```

### `specification.html`

```html
<!doctype html>
<html lang="en">
<head>
  <meta charset="utf-8">
  <meta name="viewport" content="width=device-width,initial-scale=1">
  <title>Closure rule</title>
  <link rel="stylesheet" href="./demistify/theme.css">
  <script type="module" src="./demistify/runtime.js"></script>
</head>
<body>
  <ex-nav for="rule-page">
    <nav slot="pages" aria-label="Pages">
      <a href="requirements.html">Requirements</a>
      <a href="specification.html">Specification</a>
      <a href="program-design.html">Program Design</a>
      <a href="verification.html">Verification</a>
      <a href="execution.html">Execution</a>
    </nav>
  </ex-nav>
  <ex-document id="rule-page">
    <h1>Closure rule</h1>
    <p><a href="requirements.html">Read by need</a></p>
  </ex-document>
  <script type="application/json" data-ex-records>
  [{"kind":"specification","id":"RuleClosure","title":"Show current notices","needs":["NeedClosure"],"bodyRef":"rule-body","summaryRef":"rule-summary"}]
  </script>
  <template id="rule-body">
    <p id="rule-summary" data-ex-part="rule">A route containing a closed segment displays its notice before selection.</p>
    <div class="ex-table-scroll" data-ex-part="cases">
      <table>
        <thead><tr><th>Segment</th><th>Required display</th></tr></thead>
        <tbody><tr><td>Closed</td><td>Closure notice before selection</td></tr><tr><td>Available</td><td>Normal route description</td></tr></tbody>
      </table>
    </div>
    <details><summary>Register boundary</summary><p>The register owns the closure wording.</p></details>
  </template>
</body>
</html>
```

### `program-design.html`

```html
<!doctype html>
<html lang="en">
<head>
  <meta charset="utf-8">
  <meta name="viewport" content="width=device-width,initial-scale=1">
  <title>Notice ownership</title>
  <link rel="stylesheet" href="./demistify/theme.css">
  <script type="module" src="./demistify/runtime.js"></script>
</head>
<body>
  <ex-nav for="design-page">
    <nav slot="pages" aria-label="Pages">
      <a href="requirements.html">Requirements</a>
      <a href="specification.html">Specification</a>
      <a href="program-design.html">Program Design</a>
      <a href="verification.html">Verification</a>
      <a href="execution.html">Execution</a>
    </nav>
  </ex-nav>
  <ex-document id="design-page">
    <h1>Notice ownership</h1>
    <p><a href="requirements.html">Read by need</a></p>
  </ex-document>
  <script type="application/json" data-ex-records>
  [{"kind":"program-design","id":"DesignRoute","title":"Read notices at the route boundary","specifications":["RuleClosure"],"bodyRef":"design-body","summaryRef":"design-summary"}]
  </script>
  <template id="design-body">
    <p id="design-summary">The route view reads closure wording from the seasonal register.</p>
    <div data-ex-part="sketch">
      <ex-mermaid id="notice-flow" label="Notice ownership"><script type="text/plain">
flowchart LR
  Register[Seasonal register] --> Route[Route view]
      </script></ex-mermaid>
    </div>
    <div data-ex-part="gaps">
      <ex-markdown id="route-notes" label="Design limits" src="route-notes.md"></ex-markdown>
    </div>
  </template>
</body>
</html>
```

### `route-notes.md`

```markdown
## Design limits

The register's refresh cadence has not been specified.
The diagram describes ownership, not an observed request.
```

### `verification.html`

```html
<!doctype html>
<html lang="en">
<head>
  <meta charset="utf-8">
  <meta name="viewport" content="width=device-width,initial-scale=1">
  <title>Notice check</title>
  <link rel="stylesheet" href="./demistify/theme.css">
  <script type="module" src="./demistify/runtime.js"></script>
</head>
<body>
  <ex-nav for="verification-page">
    <nav slot="pages" aria-label="Pages">
      <a href="requirements.html">Requirements</a>
      <a href="specification.html">Specification</a>
      <a href="program-design.html">Program Design</a>
      <a href="verification.html">Verification</a>
      <a href="execution.html">Execution</a>
    </nav>
  </ex-nav>
  <ex-document id="verification-page">
    <h1>Notice check</h1>
    <p><a href="requirements.html">Read by need</a></p>
  </ex-document>
  <script type="application/json" data-ex-records>
  [{"kind":"scenario","id":"CheckClosure","title":"Inspect the closed segment","specifications":["RuleClosure"],"bodyRef":"check-body","summaryRef":"check-summary"},{"kind":"observation","id":"ObservedClosure","scenario":"CheckClosure","outcome":"Partial","bodyRef":"observation-body","summaryRef":"observation-summary"}]
  </script>
  <template id="check-body">
    <p id="check-summary">Read the north route's closure notice before selecting it.</p>
    <details data-ex-part="procedure"><summary>Proposed check</summary><ol><li>Open the north route.</li><li>Read its notice before selecting it.</li></ol></details>
  </template>
  <template id="observation-body">
    <p id="observation-summary">Illustrative supplied result: notice text was present; selection order was not observed.</p>
    <p>This invented example supplies no target-build or real-path product proof.</p>
  </template>
</body>
</html>
```

### `execution.html`

```html
<!doctype html>
<html lang="en">
<head>
  <meta charset="utf-8">
  <meta name="viewport" content="width=device-width,initial-scale=1">
  <title>Supplied notice calls</title>
  <link rel="stylesheet" href="./demistify/theme.css">
  <script type="module" src="./demistify/runtime.js"></script>
</head>
<body>
  <ex-nav for="calls">
    <nav slot="pages" aria-label="Pages">
      <a href="requirements.html">Requirements</a>
      <a href="specification.html">Specification</a>
      <a href="program-design.html">Program Design</a>
      <a href="verification.html">Verification</a>
      <a href="execution.html">Execution</a>
    </nav>
  </ex-nav>
  <ex-document id="calls">
    <h1>Supplied notice calls</h1>
    <p>These fictional source descriptions do not establish execution.</p>
    <ex-call-stack id="trail-calls" label="Supplied notice calls"></ex-call-stack>
  </ex-document>
  <script type="application/json" data-ex-records>
  [{"kind":"frame","id":"ReadNotice","name":"Register.readNotice","source":"register.ts:18","responsibility":"Provide closure wording","statement":"Return the segment's supplied notice.","requirement":"NeedClosure","category":"Resolution"},{"kind":"frame","id":"ShowNotice","name":"RouteView.showNotice","source":"route-view.ts:42","responsibility":"Present the notice","statement":"Place the supplied notice beside the route description.","requirement":"NeedClosure","category":"Action","supporting":false},{"kind":"stack","id":"RouteClosure","title":"Read and present a notice","description":"A supplied source path from register to route view.","primaryNeed":"NeedClosure","frames":["ReadNotice","ShowNotice"]}]
  </script>
</body>
</html>
```

The example maps each metadata body/summary pair to exactly one template, uses all seven record kinds and supplies every forward target. `RuleClosure` owns the table/rule, `DesignRoute` owns the diagram/Markdown limits, and `CheckClosure` owns proposed procedure. The observation remains separate from that proposal.

## Prepare, then check the changed HTML

From the directory containing `trail-guide/`, using Node 26+ and the example's matching local distribution:

```sh
pnpm dlx ./trail-guide/demistify/demistify-cli.tgz prepare ./trail-guide
pnpm dlx ./trail-guide/demistify/demistify-cli.tgz check ./trail-guide/requirements.html ./trail-guide/specification.html ./trail-guide/program-design.html ./trail-guide/verification.html ./trail-guide/execution.html
```

For a hosted binding, use its `demistify-cli.tgz` URL in place of the local tarball, with the same `prepare` and `check` arguments. The main guide owns the routine one-invocation final source check; errors are corrected at their authored owners, and warnings retain that guide's advisory status.

`prepare` reads every sibling HTML file and local Markdown source, validates ownership, IDs, bodies, parts and cross-page edges, derives all output, then writes the HTML pages in place. A validation/read failure occurs before those writes. This is not an atomic filesystem transaction if an output write itself fails. Explicit `</head>` and `</body>` boundaries are required. Keep authored inputs in your normal source/version workflow.

Each output gets a generated collection meta marker and `data-ex-design-collection` JSON carrier outside `ex-document`. Keep editing the original inline `data-ex-records` and body containers, not the derived carrier. Local Markdown is embedded in a derivation copy for the prepared body; the authored `ex-markdown src` and `.md` source remain intact. Preparing again with unchanged inputs is idempotent. After any source HTML/metadata or referenced Markdown edit, prepare the directory again before the final check. Shared collection revision changes follow the prepared facts, including record bodies outside the document element.

`prepare` accepts local Markdown sources only; a missing file or remote Markdown URL is an error. The prepared body can be read without sibling Markdown fetches. Ordinary page/image links still need their accompanying files; the runtime, fonts, worker and lazy chunks still need the chosen complete binding. Preparation does not inline the library. Use the opening method and private-runtime choice already selected through the main guide.

The checker validates individual pages and their prepared carrier/markup; directory preparation establishes cross-page referential integrity. A source check cannot certify rendered layout, link interaction or supplied call execution. Report the commands you actually ran and keep routine authoring at that boundary.

## Partial work and truthful results

A needs-only collection is the complete `requirements.html` above, with only its `requirements.html` Home link in the native page list, in its own directory with the same complete library binding. There is no `execution.html`, no invented stack and no later metadata to add. Missing stages show “Not yet written”; call views have no authored path to select. For a specification-only continuation, add `specification.html` and keep the two native page lists limited to `requirements.html` first (Home), then `specification.html`; still omit unwritten design, verification and execution pages and their links. If a canonical owner file exists but has no records, its one metadata array is `[]`.

A scenario without an observation is “Not run”; do not author an observation with `outcome: "Not run"`. A supplied observation uses `Pass`, `Fail` or `Partial`. Several supplied observations can point to one scenario: give each its own ID/body/summary and retain every supplied outcome. A later Fail does not overwrite an earlier Pass or become one green overall badge. Preserve actual target/setup/limits in observation HTML; the library does not classify every result as illustrative.

Keep tables, links, inline code, Markdown, diagrams and native disclosure states in their source bodies. The readers localize IDs and local href/label/ARIA targets for projected copies; supply links to real targets and retain external/cross-page URLs. Source details exposes the full supplied body where the related view selects parts. Use the existing stage/related/call controls and automatically provided Comments; a parallel picker, custom feedback store or per-page renderer would duplicate those owners.

IF authoring Markdown carriers or changing disclosure/table composition beyond this example, load `content.md` and return the source carriers, retained forms/states and delivery method. IF changing Mermaid source beyond this example, load `diagrams.md` and return the supported family, safe source and resolving targets. These references retain their existing APIs and the main guide's validation policy.
