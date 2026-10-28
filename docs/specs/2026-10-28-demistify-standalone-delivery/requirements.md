# Demistify standalone delivery: requirements

Agents share Demistify pages into Slack, Grok and other chat viewers, and those viewers do not load the library from its CDN. On 28 October 2026 a page named `index.standalone.html` showed only its headings in Slack's file viewer. It linked `https://demistify.getagentstudio.dev/v1/runtime.js` and `theme.css`, and neither loaded, so no Markdown rendered. A page made by `bundleStandaloneHtml`, which carries the library inside the file, rendered in the same viewer. Agents have no supported way to make that second kind of file: the bundler is a source-only function, the checker rejects its output, and nothing checks a file the way a chat viewer opens it.

This document records who needs what. The [Specification](specification.md) states the observable obligations, the [Program Design](program-design.md) realizes them, and the [Verification Design](verification-design.md) plans their proof.

## Who is affected

| Class | Job | Relationship |
| --- | --- | --- |
| Authoring agent | Produces a Demistify page for a reader, in any repository, usually without a Sunstack tools checkout | Direct user of the CLI and the skills |
| Reader | Opens the shared page in Slack, Grok, a chat attachment viewer, or from disk without network | Indirect beneficiary; never runs a command |
| Owner (Shravan) | Shares pages, merges PRs, releases Demistify | Decision authority; stakeholder |
| Demistify maintainer agent | Changes the library and proves it in this repository | Direct user of the CLI from source and of `verify-demistify` |

## Capabilities

The owner named these parts on 28 October 2026: "pnpm dlx demistify verify and standalone helpers", "allow the cli to help", "what skills will help", and "seperate pr for veify, sperate for skill chagnes".

| Key | Owner's name | What users get |
| --- | --- | --- |
| `cli` | the CLI run with `pnpm … dlx demistify` | an agent in any repository runs the Demistify tools with one command, for the release its page uses |
| `bundle` | standalone helpers | one self-contained HTML file a chat viewer or offline reader can open |
| `verify` | `demistify verify` | proof, before sharing, that a delivered file renders where its reader opens it |
| `skills` | skill changes | agents choose the right delivery and use the commands correctly without improvising |

## Needs

| U | Capability | Need or outcome | Evidence | Authority | Priority |
| --- | --- | --- | --- | --- | --- |
| U1 | `cli` | An agent runs the CLI with `pnpm --package=https://demistify.getagentstudio.dev/v1/cli.tgz dlx demistify <command>`, with no repository checkout. | Owner: "pnpm --package=https://demistify.getagentstudio.dev/v1/cli.tgz dlx demistify bundle …. this is what i want" | authorized | must |
| U2 | `cli` | The package that displays pages and the CLI are separate; neither depends on the other at install time, and adding the CLI does not make the display package bigger. | Owner: "the pcakge and cli to run can be seprate right? one doesnt ahve to make the otehr big?", "one doesnt nee dto depl on other" | authorized | must |
| U3 | `cli` | The CLI never quietly builds or checks with a library release other than the one the page binds. | Owner chose the CDN-archive route after the recommendation that it rules out a bundler/library/page mismatch; the working bundle's author had to download and hash-check the release by hand | authorized | must |
| U4 | `bundle` | One command turns an authored page into one HTML file that renders with no network: library, theme, fonts, Markdown and graph data inside it. | Slack screenshot of the broken page; the working address-mismatch bundle; the improvised `bundle.ts` script in the waypoint-workflows session | authorized | must |
| U5 | `bundle` | `demistify check` accepts a correct bundle instead of rejecting it, and rejects a file that claims to be standalone but is not. | Recorded finding F6: `check` reports 1 error and 121 warnings on correct `bundleStandaloneHtml` output; the misnamed `index.standalone.html` | authorized | must |
| U6 | `verify` | An agent proves a delivered file renders when opened from disk with network blocked, and sees exactly what failed when it does not. | The broken page was shared without any check that could have caught it; `check --render` serves over HTTP with network allowed | authorized | must |
| U7 | `skills` | Skills route agents to a bundle when the reader opens the page in a chat viewer, an attachment or offline, teach the commands and limits, and require the offline check before delivery. | Owner: "we need to provide a skill -> reference and help for agents on how to do it well"; `presentation-html` currently says the bundle "exists only as the source API" and forbids inventing a command | authorized | must |
| U8 | `skills` | The skills start from the latest `presentation-html`. | Owner: "yes get latest presetnation hytml" | authorized | must |
| U9 | `skills` | The change arrives as separate pull requests: this design, the CLI with bundling, verification, and skill changes. | Owner: "seperate pr for veify, sperate for skill chagnes", "make worktrees and prs" | authorized | must |

## Boundary

Inside: a separate CLI package and its release archive at `/v1/cli.tgz`; bundling, checking and verifying as CLI commands; bundle-aware checking; the library's guide, `presentation-html` and `verify-demistify`.

Protected: the runtime, theme, chunks, fonts and worker the display package publishes keep their current contents and URLs, and existing CDN-linked pages keep working unchanged.

Non-goals:

- publishing to npm or any registry other than the Demistify CDN;
- inlining pictures or separate Mermaid files into a bundle (the bundler keeps failing clearly on them);
- emulating Slack's or Grok's exact security policy (blocking all network is the stricter check);
- bundling a whole collection of pages in one command, or rewriting links between pages;
- downloading browsers for `verify`;
- merging, tagging, deploying or releasing by agents.

Proposed limits, awaiting the owner's confirmation in review of this design: the non-goals above, Node 26 or later, and pnpm 11 or later.

## Open questions

- Slack's and Grok's actual Content Security Policies are unobserved. The inference that they block other origins rests on the two files' different results in Slack.
- Whether chat viewers block `blob:` workers or images is unknown. The owner reported the bundle works in Slack, and it boots from `blob:` modules, so those ran. Its four Mermaid diagrams were not observed separately there.
