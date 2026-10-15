# Session reconnection

Synthetic design example — not a claim about any product

Today, closing a transport also destroys the logical session. Move session lifetime into the session store so a new transport can attach to the same session.

## Current architecture

::diagram{src="fixtures/session.d2" title="Current architecture"}

## Proposed architecture

::diagram{src="fixtures/proposed.d2" title="Proposed architecture"}

:::decision
The session store owns session lifetime; the reconnect worker owns reconnection attempts.
:::

## Alternatives

| Alternative                | Gain                                        | Cost                                                 |
| -------------------------- | ------------------------------------------- | ---------------------------------------------------- |
| Reconnect inside transport | Fewer components to introduce               | Session lifetime remains coupled to a connection     |
| Reconnect owned by session | One session survives replacement transports | Attachment and cleanup rules need explicit ownership |

:::timeline
1. **Define attachment ownership.** Specify one active transport per session and refusal of stale attachments.
1. **Move lifetime into the store.** Add a reconnect worker and replace the transport-local socket cache.
1. **Exercise disconnect and reconnect.** Verify that the same session survives and a second attachment is refused.
:::

:::risk
Two transports may race to attach. The session store must accept one active transport per session and refuse the stale attachment.
:::
