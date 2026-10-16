# Full reference as a reusable explorer

Open `sol-iterated.html` and `luna-iterated.html` independently after the normal lab build. Both use the same `planning.json` and independently written HTML. Initial and first accepted outputs remain verbatim. No model wrote application JavaScript, CSS or a duplicate graph.

Fresh Sol (`gpt-6.1-sol`, medium) and Luna (`gpt-6-luna`, high) read the same four input files, totaling 2,141 source tokens. They received a graph inventory and referenced the complete supplied JSON without reading its entire payload into their author context. The checker did read and validate that payload. Initial wrappers measured 222 and 219 tokens. Both first checks passed with zero errors and warnings. One composition-feedback iteration moved each reading guide inside the island after its title; final wrappers measure 223 and 212 tokens. Each new check passed on its first call.

The referenced JSON is 173,548 tokens, 776,142 bytes (97,429 gzip). It is reused content, not free content or model-generated output. The one-time input adapter is separately counted. Including it and the referenced content does not establish a first-migration total-token saving over the original 174,515-token HTML. Small author context is possible here because structured source content already exists.

The normalized corpus contains 49 nodes, 329 current edges and 21 proposed edges. A node may describe a proposed shared reference rather than a skill. Proposal status does not mean implemented. `source-manifest.json` pins the reference revision and hash. The original application's browser visual could not be opened under the host's file-protocol policy; no mirror or alternate browser bypass was used.

The library adds one lazy `ex-explorer` component. Search, history, current/proposed views, hover tracing, arrow-to-row selection, incoming consumers, supplementary tables and source quotations are shared behavior. Node/edge sets matched the frozen reference's pure composition function in 100 cases. Real Chrome checks exercised representative interactions and desktop/narrow layouts. This is not pixel parity, a complete accessibility audit, arbitrary cyclic-graph routing proof, or exhaustive interaction acceptance.

The root WIP results document has the actual screenshots, loading samples, local delivery trials and remaining gaps. Machine paths in receipts are normalized; exact HTML and JSON are unchanged. Complete assignment/tool/hidden context and billed usage were not captured in this round.
