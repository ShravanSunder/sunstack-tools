# Demistify versus raw HTML authoring

Demistify reduced delivered reader-source size in six single-run authoring trials. Total author-session usage varied by model and effort, and every condition still needed content review. These measurements are an exploratory comparison, not a reliability estimate or a claim that a library always saves model tokens.

Fresh Sol and Luna sessions created the same two-page fictional Session recovery review with either ordinary HTML/CSS/JavaScript or Demistify HTML islands. A follow-up repeated both Luna conditions at xhigh effort. Native runs used Demistify 1.2.0, source `58a856f301dc3318d394ee783243c1932de06b85`, and its matching API guide.

## Delivered reader source

Counts include the final HTML, authored CSS/JavaScript and any generated Markdown embedding inside that HTML. Supplied images and sibling source files are held constant and excluded. Shared library assets are separate.

| Profile     | Approach        | Source tokens |  Bytes |
| ----------- | --------------- | ------------: | -----: |
| Sol, medium | Raw HTML/CSS/JS |         8,477 | 31,717 |
| Sol, medium | Demistify       |         3,074 | 13,592 |
| Luna, high  | Raw HTML/CSS/JS |         5,963 | 22,038 |
| Luna, high  | Demistify       |         3,487 | 14,997 |
| Luna, xhigh | Raw HTML/CSS/JS |        15,947 | 55,590 |
| Luna, xhigh | Demistify       |         3,315 | 14,027 |

Within these pairs, Demistify reader-source tokens were 63.7%, 41.5% and 79.2% lower. These are total reader-code comparisons. HTML alone was not always smaller: Luna high delivered 12,862 raw HTML bytes versus 14,997 Demistify HTML bytes, while its raw condition also authored 9,176 CSS/JS bytes.

The library moves reusable rendering and interaction out of each authored page. Its CDN runtime, stylesheet, fonts and lazy chunks still have a delivery cost. Reader transfer, compression and cache effectiveness were not measured here. Source tokens use `js-tiktoken@1.0.21` with `o200k_base`; they are counts of files, not billed generation tokens.

## Actual author-session usage

Host counters include guide/tool reading, author self-checks and repairs before the first delivered handback. Root setup, library development, independent review and browser assessment are excluded. One metadata-only identity follow-up is included in each run except Luna xhigh Demistify, which needed two. Cached input is cumulative reused context across calls; it is not fresh input. Reasoning tokens are a subset of output.

| Profile     | Approach  | Uncached input | Cached input | Output | Reasoning subset | Time to last artifact |
| ----------- | --------- | -------------: | -----------: | -----: | ---------------: | --------------------: |
| Sol, medium | Raw       |         63,630 |      558,080 | 11,983 |              541 |               158.0 s |
| Sol, medium | Demistify |         74,549 |      736,256 |  6,883 |              474 |                98.7 s |
| Luna, high  | Raw       |         65,860 |      640,640 | 11,899 |            2,155 |               100.3 s |
| Luna, high  | Demistify |         59,101 |    2,103,168 | 19,230 |            8,612 |               358.8 s |
| Luna, xhigh | Raw       |        112,406 |    1,585,792 | 59,115 |           18,223 |               562.8 s |
| Luna, xhigh | Demistify |         74,008 |    3,400,192 | 36,162 |           24,062 |               324.0 s |

Sol used 42.6% fewer output tokens with Demistify. Luna high used 61.6% more; Luna xhigh used 38.8% fewer. Both xhigh conditions used more output than their high counterparts. A single run per condition cannot isolate a causal effect of effort. Dollar charges were unavailable, so these counters do not establish billed savings.

Time runs from session creation to the last authored HTML/CSS/JS or author-note write, not exact handback time. Identity follow-ups are excluded from elapsed time. The full commissioning text was reissued for xhigh rather than byte-identical; the common brief, supplied inputs, approach restrictions, release and guide were held constant.

## What the reader checks found

Root inspected frozen handbacks in Brave at desktop and 375px widths over an HTTP preview. Initial drafts and author self-repairs were retained; Root did not repair the compared artifacts.

| Condition             | Observed strengths                                                                                                            | Remaining limitations                                                                                                                                                                                   |
| --------------------- | ----------------------------------------------------------------------------------------------------------------------------- | ------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------- |
| Sol medium, raw       | Graphical models, proof choices, independent folding, gallery filtering and narrow containment                                | Authors supplied their own SVG geometry, CSS and interaction code                                                                                                                                       |
| Sol medium, Demistify | Graphical models/sequence, proof switch, tree deep links, source inspection, gallery state and exact literal                  | Verbose gallery titles; introductory prose kept its gallery in embedded layout                                                                                                                          |
| Luna high, raw        | Gallery filters/views worked                                                                                                  | Main page widened to 520px at 375px; literal lost its final newline; Brave blocked embedded source/diff frames in the HTTP preview                                                                      |
| Luna high, Demistify  | Controls, diagrams, source inspection and narrow containment worked                                                           | One paragraph reversed the patch's meaning; duplicate Chosen badge; literal lost its final newline                                                                                                      |
| Luna xhigh, raw       | Graphical HTML ownership views, SVG sequence, independent folding, proof choices, gallery state and narrow containment worked | Literal and displayed diff lost their final newline; Current excerpt panel described `missing` where the supplied pre-change code returns `undefined`; filtered group label retained its original count |
| Luna xhigh, Demistify | Controls, real flow connectors/sequence, source inspection, exact Markdown/diff/literal and narrow containment worked         | Introductory prose again selected embedded gallery layout, leaving unused desktop space; semantic source identifiers were not consistently code-formatted                                               |

Green structural checks did not establish content fidelity. The xhigh Demistify author repaired three initial note-nesting errors before handback. The raw xhigh author restored a missing ownership table and added an explicit Current/Proposed comparison. More effort did not remove the need for independent source comparison and visual review.

## Reproduction scope

The fixed task required four ownership/risk findings, before/after ownership models, the supplied Markdown table/tasks/code/sequence, an exact two-file diff with operation-specific notes, a literal drawing, independent hierarchical fold/navigation, a linked walkthrough and four grouped screenshots with filters and Review/Grid views. Raw authors could choose ordinary HTML/CSS/JS and renderer dependencies; they did not have to recreate every native inspector or presentation-player affordance.

Each run used a fresh conversation with no inherited experiment history. Native authors explicitly received the matching API guide and the same workflow source candidate. Automatic installed-skill discovery was not tested. Original input files were identical across arms. The fictional recovery behavior and tenant isolation were not executed or proven.

The reader HTML uses shared CDN assets and ordinary image paths; sibling Markdown was embedded for native direct-file delivery. No reader-local application server is part of that design. These browser observations use HTTP and do not prove direct-file execution. They also do not measure reader comprehension, accessibility conformance, WAN performance or general model reliability. This compares an HTML-artifact approach, not Claude.ai or an Anthropic model. The earlier HTML-versus-object-JS prototype answered a different question and is excluded.

[Machine-readable counts and file hashes](demistify-authoring-2026-10-17.json) accompany this report. Aggregate measurements are published here; author artifacts and host session records remain outside this repository.
