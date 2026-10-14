# Brief: deep dive — the session registry

Audience: an engineer about to change the session registry. They need the overview in a minute and every detail of the component within reach: interface, states, storage, failure behaviour, configuration, metrics and known issues. Illustrative component, dated 2026-10-14.

Opening summary: The session registry keeps a client's session alive across dropped connections. It maps a session id to live state for a grace window, so a reconnecting client resumes instead of starting over. It runs in every gateway instance and stores sessions in Redis.

## Parts

- **Registry API** — the only entry point. `create(clientId)`, `resume(sessionId, socket)`, `touch(sessionId)`, `expire(sessionId)`.
- **Session store** — Redis hash per session; key `sess:{id}`; fields: client id, state, last seen, version. TTL equals the grace window.
- **Socket binder** — attaches and detaches the live socket for a session; at most one socket per session.
- **Expiry sweeper** — background task every 30 s; ends sessions whose grace window passed and emits `session.ended`.
- **Event publisher** — publishes `session.started`, `session.resumed`, `session.ended` to the event bus.

Relationships: Registry API → Session store (reads/writes); Registry API → Socket binder (attach/detach); Expiry sweeper → Session store (scans); Expiry sweeper → Event publisher (ended); Registry API → Event publisher (started/resumed).

## Session states

- **active** — a socket is attached.
- **detached** — the socket dropped; the grace window is running.
- **ended** — grace window passed or the client logged out. Final.

Transitions: active → detached (socket drop); detached → active (resume within grace window); detached → ended (grace window passed); active → ended (logout).

## Resume flow

1. Client reconnects and sends its session id.
2. Gateway calls `resume(sessionId, socket)`.
3. Registry reads `sess:{id}` from Redis.
4. If the state is detached and the version matches, the binder attaches the socket, state becomes active, version increments.
5. Registry publishes `session.resumed` and returns the session.
6. Otherwise the gateway tells the client to start a new session.

## Key code

```ts
export async function resume(id: string, socket: Socket): Promise<Session | undefined> {
  const session = await store.read(id);
  if (!session || session.state !== 'detached') return undefined;
  const attached = await store.compareAndSet(id, session.version, { state: 'active' });
  if (!attached) return undefined;
  binder.attach(id, socket);
  events.publish('session.resumed', { id });
  return attached;
}
```

Notes on that code: line 2 is a network round trip to Redis; line 4 uses the version to stop two tabs resuming the same session; line 5 returns undefined when another tab won, which looks like a miss to the caller.

## Configuration

| Setting | Default | Meaning |
| --- | --- | --- |
| `SESSION_GRACE_SECONDS` | 300 | How long a detached session can resume |
| `SWEEP_INTERVAL_SECONDS` | 30 | Expiry sweeper period |
| `REDIS_URL` | — | Session store location |
| `MAX_SESSIONS_PER_CLIENT` | 5 | Oldest session ends when exceeded |

## Metrics (last 7 days, production)

- Resume success rate: 97.8%
- p95 resume latency: 4.1 ms
- Sessions ended by sweeper per hour: 1,240
- Two-tab conflicts per day: 37

Resume success by cause of disconnect: network blip 98%, deploy 96%, instance crash 94%, client sleep over 5 min 0%.

## Failure behaviour

- Redis unavailable: `create` fails closed (client gets an error); `resume` returns undefined, so clients start new sessions.
- Sweeper lagging: sessions live longer than the grace window; harmless except memory.
- Two tabs resume at once: one wins by version; the other starts a new session.

## Decisions already made

- Redis over in-memory: survives deploys and is shared across gateway instances; costs ~2 ms per resume.
- Compare-and-set on a version rather than a lock: no lock service needed; the loser sees a miss.

## Known issues

- A two-tab conflict looks identical to an expired session in logs.
- The 5-minute grace window is global; some tenants want longer.
