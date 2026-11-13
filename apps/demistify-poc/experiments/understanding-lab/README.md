# Authored needs and call paths

The pages keep their original illustrative need, rule, design, procedure and observation text as visible HTML. They are examples, not claims about a shipped product.

- `requirements.html`: user needs and stable `body-U1` through `body-U6` anchors.
- `specification.html`: authored rules and cases.
- `program-design.html`: owners, diagrams and design gaps.
- `verification.html`: supplied procedures and illustrative observations.
- `execution.html`: the retained `ex-call-stack` reader, with stack/call selection, supporting calls, keyboard navigation and Comments.

The public `prepareDesignPages` entry validates inline records, body IDs and forward references before writing the prepared collection. The call-stack reader consumes that snapshot. Need links resolve to the recorded owner page and body ID. A frame's source address remains an exact caption.

A collection must contain at least one need; call paths may be absent. Missing or wrong-kind declared references reject preparation. Calls start at the first authored stack and frame unless the address selects another. An unmapped need keeps its explicit gap while available calls remain usable.

Reusable rendering and styles belong to `packages/demistify`; these pages own their text and layout. Build from the repository root with `pnpm build`. The independent corner and color studies remain inspectable in `corners.html` and `color.html`.
