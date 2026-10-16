# Sidebar inspection and public authoring documentation

The shared inspector now keeps the original island in the page when opening a sidebar. Code comments use a neutral gray background with a severity accent, distinct from added/removed code. Corrected public examples and version-matched authoring documentation are deployed. Sunstack presentation-html is a separate skill-delivery task; track it and remaining experiments in [the owner checklist](2026-10-16-owner-todos.md).

## Open

- [Latest Sol/Luna examples](https://demistify-assets-cli-experiment.askluna.workers.dev/examples/2026-10-16-sol-luna-v3/index.html)
- [Luna review](https://demistify-assets-cli-experiment.askluna.workers.dev/examples/2026-10-16-sol-luna-v3/luna-review.html)
- [Interactive composition](https://demistify-assets-cli-experiment.askluna.workers.dev/examples/2026-10-16-sol-luna-v3/inspection.html)
- [Luna explorer](https://demistify-assets-cli-experiment.askluna.workers.dev/examples/2026-10-16-sol-luna-v3/luna-explorer.html)
- [Canonical component guide](https://demistify-assets-cli-experiment.askluna.workers.dev/demistify/experimental/c96a72e/authoring/SKILL.md)

Use `theme.css` and `runtime.js` from the same exact root:

`https://demistify-assets-cli-experiment.askluna.workers.dev/demistify/experimental/c96a72e/`

The optional guide and six references are under `authoring/` beside that root. The Sunstack skill can retrieve this canonical API rather than carry another component catalog. Public asset loading does not publish an explainer’s own content; a trusted complete local bundle remains appropriate for sensitive or offline content.

## Behavior and ownership

`islands/inspector.ts` chooses the inspection shell. Sidebar content is an independent rendered copy; the main DOM, IDs, handlers and disclosure state stay in place. `islands/inspection-copy.ts` namespaces copied IDs/references and preserves native control values. Components reconnect their own rendered controls. Explorer reconstruction stays in its controller, including graph node/edge handlers; there is no second graph data model. Fullscreen still temporarily enlarges the real content and restores it on close.

Tabs, trees and sliders start from the copied view and then have independent local UI state. This is inspection, not a synchronized editor. Explorer graph selection remains owned by its controller: choosing a node in the inspected graph updates both graphs and the corresponding details table. Native disclosure details are revealed in inspection without changing the main page’s saved state.

Code headers wrap safely and reserve button space only where buttons exist. Review-note backgrounds use the neutral theme token; the small severity border/icon keeps the finding’s meaning. Green/red remain code-change semantics. Circular sidebar/fullscreen icons continue to come from the library.

Navigation remains the shared left outline generated from headings. The review showed eight h2 sections and nine outline links, with the sidebar open and no outer overflow. Collections use ordinary relative page links and accessible breadcrumb markup; they do not require an app router.

## Proof

Observed the original public failure: main code had zero lines, sidebar had eleven, and the main view held an “open in the panel” placeholder. Notes used red, amber and blue backgrounds. The repaired hosted review had eleven lines in each view, zero placeholders, no duplicate IDs, and one neutral gray note background with distinct severity accents. A local light-theme check showed the corresponding light gray background; fullscreen had eleven lines/five notes and closing restored all eleven lines.

The hosted interactive composition retained Tree in the main view while its panel switched to Comparison; moving the panel slider changed it to 51 while the main slider stayed at 50. A copied tree branch collapsed without changing the main tree. Both Mermaid views held SVG and the copy retained its linked node. Actual issue-panel inspection reported none.

The final hosted explorer used the c96a72e runtime. Opening its graph gave 22 nodes in both main and sidebar; selecting principle-define-the-domain-first in the sidebar gave one node in each and one corresponding details row, with no authoring issue panel. This specifically checks the handler-loss regression of a raw DOM clone.

- Existing suite: 34 tests / 9 files passed, exit 0; no additional pedantic suite.
- `pnpm check`: type-aware lint, formatting and strict TypeScript passed.
- `pnpm build`: library and demo build passed.
- New example sets: each six HTML files checked with zero errors and warnings.
- Final public protocol: 95 files returned matching staged bytes and CORS; old immutable roots retained. [Protocol receipt](evidence/2026-10-16/interactive-public-protocol.json).
- Earlier v2 caching probe: empty 304 with CORS; missing asset 404/no-store. [Receipt](evidence/2026-10-16/inspection-public-protocol.json).
- [Browser observations and limits](evidence/2026-10-16/inspection-browser-observations.json).

![Main review and sidebar both visible with gray comments](evidence/2026-10-16/hosted-review-sidebar.png)

![Independent composition controls](evidence/2026-10-16/hosted-independent-composition.png)

![Explorer selection retained in both views](evidence/2026-10-16/hosted-explorer-selection.png)

## Delivery and continuation

Runtime source commits: 16ac23d (shared copy/note repair), c96a72e (explorer interaction correction). Final Cloudflare deployment: 46686e68-6b07-4173-8ec0-a53a9b4cd897. The v1 and v2 paths are retained unchanged; new pages use v3. These remain Lead derivatives of earlier Sol/Luna author outputs, not fresh author runs.

`apps/demistify-poc/scripts/prepare-cloudflare-public.ts` assembles the three explicitly named frozen releases, verifies source hashes and refuses changed frozen targets. Historical library files missing from the current build are retrieved from their exact public URL and checked against the stored SHA256 before publication. Sample graph content is reused through a repository-local symlink; published JSON is ordinary byte content. This bounded trial is not general archival storage or a production release service. Never put raw operator logs, auth material or account identifiers in the stage.

Unverified: the attempted responsive viewport override did not alter the target tab, so that probe does not establish the narrow-sheet path. It was reset. Native window/selected-tab developer inspection also failed independently of the page. No latency result is derived from those tool failures. Full original-explorer parity, field/cold-load measurements, production release strategy and standalone checker distribution remain separate checklist items.

## Fresh presentation-skill characterization

New independent Sol xhigh and Luna high reference-application cases are in `apps/demistify-poc/experiments/agent-written/round-15-presentation-skill/`. These differ from the earlier Lead-rebound public sample pages. Each model read the complete task skill, delivery reference and matching guide/code reference, verified from supported live tool output. Luna initially had two invalid-tone errors; Sol had one Markdown heading-spacing warning. One fresh correction request each produced checker 0 errors/0 warnings. Source HTML costs are 1,099 and 1,416 o200k_base tokens. API documentation, workflow guidance, brief and runtime costs are separate in measurements.json.

Chrome observed actual Markdown, 1/2 Mermaid SVG, heading navigation and code on the two corrected pages; Luna retained 9 main code rows alongside 9 sidebar rows. Screenshots are saved with those source pages. Narrow layouts and cold/warm latency remain unverified. The original Sol revision artifact's file-URL denial was not bypassed; the independent fresh Sol page was opened HTTP-first. A content-only Luna revision preserved graph source, IDs, links and the asset binding byte-for-byte outside two requested title replacements, while omitting one required reference read. These are bounded observations, not first-pass reliability or a model ranking.

The Sunstack skill source and proof are pushed at `f1898058` on `feat/presentation-html-cloudflare` in the canonical [ShravanSunder/sunstack repository](https://github.com/ShravanSunder/sunstack/tree/feat/presentation-html-cloudflare). Independent spec and implementation review both returned great; all nine implementation checks are complete and no accepted finding remains open. Installed-plugin refresh and default-branch merge are outside that delivery boundary. The owner checklist retains the broader explorer/performance/distribution obligations.

Deferred library observation: source text normalization strips the first diff context row's leading space. Row numbers are unchanged; the first row can align one column left. This cosmetic issue is source-observed and outside the inspector repair; no new correction is claimed here.

## Cloud-agent pickup

- Library/experiments: `ShravanSunder/sunstack-tools`, branch `feat/demistify-distribution-authoring-experiments`; fresh frozen authoring evidence commit `502fc82`. From the monorepo root, `pnpm install`, `pnpm build`, then `pnpm preview --port 4174`; open `/experiments/agent-written/round-15-presentation-skill/index.html`. Each model's page opens independently and loads the public library directly.
- Skill: `ShravanSunder/sunstack`, branch `feat/presentation-html-cloudflare`, commit `f18980581a009e8f985ce927105e66b564275a4f`. Start at `plugins/sunstack-dev/skills/presentation-html/SKILL.md` and `docs/changelog/references/2026-10-16-presentation-html-proof.md`. Source packaging validates; this is a feature-branch delivery, not installed availability or default-branch merge. Two unrelated local WIP directories were excluded from the commit.
- The intent remains compact content/composition with reusable library rendering. Keep actual authored HTML/data/documentation inputs separate from shared runtime and billed/context counters. The first measured pages each needed one narrow checker-feedback correction; preserve those failures and limits when iterating.
- Continue the open checklist items: original explorer parity, repeated/model/context validation, narrow layout, cold/warm and interaction performance, minimum-install checker distribution and production version-retention strategy. Existing public assets are an experimental pin; no stable alias/retention or standalone checker release has been promised.
- Local preview commands in the skill bind explicitly to `127.0.0.1`. Independent review caught the earlier all-interface default; the corrected CLI, identical HTTP response and actual loopback listener were verified. No credentials, account metadata or raw private action streams are in the source commits.
