# Session reconnection proposal

Audience: engineers deciding whether to accept this design.
A transport owns one live WebSocket. A session owns running work. A registry indexes sessions by stable ID. On reconnect the client sends its session ID; the gateway looks it up, rejects a missing/expired session, otherwise attaches the new socket to the existing session. Disconnecting a socket should not cancel work.

Current design: the socket owns its session; losing the socket loses the reference. Proposal: a process-local Map registry separates session and transport lifetimes. It helps network reconnects but the Map is lost on process restart; do not claim restart durability.

Review finding: this illustrative function deletes an expired entry and then still attaches it. Return undefined immediately after deletion. Lines start at 40.

```diff
 export function resume(id: string) {
-  return sockets.get(id)?.session;
+  const session = registry.get(id);
+  if (!session) return undefined;
+  if (session.expiresAt < Date.now()) {
+    registry.delete(id);
+  }
+  return session.attach(currentSocket());
 }
```

Options: use the in-memory Map for a bounded reconnect experiment (simple, fast, no restart survival), or add a persistent shared registry (survives process replacement but adds an external dependency and consistency work). No performance measurements have been supplied. Proposed decision: first fix expiry, then validate reconnect behavior and decide persistence separately.
