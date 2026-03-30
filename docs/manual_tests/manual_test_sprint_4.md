# Sprint 4 — Manual Test Checklist (Failure Detection)

## Goal

Verify that the controller detects missing heartbeats and marks nodes Offline.
Also verify that nodes recover to Healthy when restarted.

This sprint only validates failure detection. Service reassignment is Sprint 5.

---

# Test Environment

## Terminals

Terminal 1 — Controller:
cargo run -p homeedge-controller

Terminal 2 — Agent A:
cargo run -p homeedge-agent

Terminal 3 — Agent B:
cargo run -p homeedge-agent

Terminal 4 — API:
export API=http://127.0.0.1:8080

Get node IDs from agent logs:
export NODE1=<node_id>
export NODE2=<node_id>

Useful checks:

curl $API/nodes | jq

---

# Test 1 — Node marked Offline after heartbeat timeout

## Steps

Start controller:
cargo run -p homeedge-controller

Start two agents:
cargo run -p homeedge-agent
cargo run -p homeedge-agent

Verify both Healthy:
curl $API/nodes | jq

Stop one agent (simulate failure):
CTRL+C in Agent B terminal

Wait ~30–40 seconds (heartbeat timeout).

Check nodes:
curl $API/nodes | jq

## Expected result

Controller log:

WARN node status changed node_id=... from=Healthy to=Offline

API:

One node Healthy
One node Offline

PASS if:

Exactly one node becomes Offline
Controller logs transition once

---

# Test 2 — Node recovery after restart

## Steps

Restart stopped agent:
cargo run -p homeedge-agent

Wait ~10 seconds.

Check nodes:
curl $API/nodes | jq

## Expected result

Controller log:

INFO node status changed node_id=... from=Offline to=Healthy

PASS if:

Node returns to Healthy
Heartbeat resumes
Transition logged once

---

# Docker note

Original plan referenced docker stop. Since agents currently run via cargo run, simply stopping the process simulates failure.

Docker will be introduced later when agents run as containers.

---

# Sprint 4 Acceptance Criteria

Sprint 4 is complete when:

✓ Missing heartbeats mark node Offline
✓ Offline transition logged
✓ Node recovers on heartbeat
✓ Recovery transition logged
✓ No duplicate transition logs
✓ System remains stable

---

# Quick Acceptance Run

1 Start controller
2 Start two agents
3 Verify both Healthy
4 Kill one agent
5 Verify Offline after timeout
6 Restart agent
7 Verify Healthy recovery

If this passes, Sprint 4 failure detection is complete.
