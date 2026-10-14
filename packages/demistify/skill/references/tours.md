# Tours and step-through

A route step may go to another page: `<ex-step to="registry.html#resume">…</ex-step>`; the tour continues there and comes back. Link the pages to each other with ordinary Markdown links too.

A step whose `to` is a Mermaid node id lights that node and its links and dims the rest, so a route can walk a diagram one node at a time.

`<ex-route label steps="a b c">` is the short form when steps need no narration.
