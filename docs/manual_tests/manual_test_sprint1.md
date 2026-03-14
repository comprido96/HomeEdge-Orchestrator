# Sprint 1 Manual Test Checklist

## Preconditions

Build the workspace:

```bash
cargo build
```

Ensure no previous controller or agent processes are running.

---

## Test 1 — Controller starts correctly

**Step**

```bash
cargo run -p homeedge-controller
```

**Expected result**

```
homeedge-controller listening addr=127.0.0.1:8080
```

**Verify port is reachable**

```bash
curl http://127.0.0.1:8080/nodes
```

Expected response:

```json
{"nodes":[]}
```

Controller state should be empty.

---

## Test 2 — Agent registration

**Step** *(new terminal)*

```bash
cargo run -p homeedge-agent
```

**Expected agent logs**

```
homeedge-agent starting
registered node <UUID> with controller
```

**Verify controller state**

```bash
curl http://127.0.0.1:8080/nodes | jq
```

Expected response:

```json
{
  "nodes": [
    {
      "id": "<same UUID>",
      "status": "registering",
      "last_heartbeat": null,
      "capabilities": [...]
    }
  ]
}
```

Status should initially be `registering`.

---

## Test 3 — Heartbeat lifecycle transition

Wait approximately 5 seconds.

**Expected agent logs**

```
heartbeat sent for node <UUID>
```

**Verify controller state**

```bash
curl http://127.0.0.1:8080/nodes | jq
```

Expected response:

```json
{
  "nodes": [
    {
      "id": "<UUID>",
      "status": "healthy",
      "last_heartbeat": "<recent timestamp>",
      "capabilities": [...]
    }
  ]
}
```

Status must now be `healthy`. This validates the `registering → healthy` lifecycle transition.

---

## Test 4 — Assignment polling

**Expected agent logs** (every ~5 seconds)

```
assignments for node <UUID>: []
```

**Verify controller endpoint manually**

```bash
curl http://127.0.0.1:8080/assignments/<UUID>
```

Expected response:

```json
{
  "node_id": "<UUID>",
  "assignments": []
}
```

An empty list is correct for Sprint 1.

---

## Test 5 — Registration retry ⚠️

This validates the backoff logic and is one of the most important tests.

**Step 1** — Stop the controller:

Press `CTRL+C` in the controller terminal.

**Step 2** — Start the agent first:

```bash
cargo run -p homeedge-agent
```

Expected agent logs:

```
registration failed ... retrying in 1s
registration failed ... retrying in 2s
registration failed ... retrying in 4s
```

**Step 3** — Start the controller while the agent is retrying:

```bash
cargo run -p homeedge-controller
```

**Expected outcome**

The agent eventually logs:

```
registered node <UUID> with controller
```

This proves registration recovery works correctly.

---

## Test 6 — Multiple agents *(optional but recommended)*

Start a second agent in a new terminal:

```bash
cargo run -p homeedge-agent
```

**Verify controller state**

```bash
curl http://127.0.0.1:8080/nodes | jq
```

Expected response:

```json
{
  "nodes": [
    { "id": "...", "status": "healthy" },
    { "id": "...", "status": "healthy" }
  ]
}
```

This validates:

- Multiple registrations work independently
- State isolation between nodes works
- The assignments map handles multiple nodes correctly

---

## Test 7 — Heartbeat recovery

**Steps**

1. Kill the agent process (`CTRL+C`).
2. Wait 10–15 seconds.
3. Restart the agent: `cargo run -p homeedge-agent`

**Expected agent logs**

```
registered node <UUID> with controller
heartbeat sent for node <UUID>
```

**Verify controller state**

```bash
curl http://127.0.0.1:8080/nodes | jq
```

Node status should return to `healthy`.

> This test prepares the ground for Sprint 4 stale-node detection.

---

## Success Criteria

Sprint 1 is considered complete when all of the following pass:

| # | Criterion |
|---|-----------|
| 1 | Controller starts and responds to `/nodes` |
| 2 | Agent registers successfully |
| 3 | Heartbeats update node status to `healthy` |
| 4 | Assignment polling returns empty list without errors |
| 5 | Registration retry with backoff works |
| 6 | Multiple agents register and remain isolated |
| 7 | Node lifecycle transitions are correct (`registering → healthy`) |

---

## Quick Regression Script

A fast sanity check you can run at any time while the controller and at least one agent are running:

```bash
curl http://127.0.0.1:8080/nodes
curl http://127.0.0.1:8080/assignments/<NODE_ID>
```

If both return `200 OK` with valid JSON, Sprint 1 plumbing is intact.
