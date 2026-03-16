# Sprint 3 — Manual Test Checklist (Reconciliation Loop)

## Goal

Validate that the agent enforces desired state, runs simulated services, reconciles drift, and reports service health via heartbeat.

Sprint 3 is complete when the agent behaves like a minimal orchestrator:

* Desired state is pulled from controller
* Local runtime converges automatically
* Services start/stop correctly
* Health is reported upstream

---

# Test Environment

## Prerequisites

## Terminal commands reference

Terminal 1 — Controller:

```bash
cargo run -p homeedge-controller

Terminal 2 — Agent:

```bash
cargo run -p homeedge-agent
```

Terminal 3 — API:

Set base URL:

```bash
export API=http://127.0.0.1:8080
```

Get node id (from agent log):
```bash
# Example:
export NODE=07373f0f-a44f-4d34-a0c9-2e915f4b2322
```

Useful helpers:
```bash
curl $API/nodes | jq
curl $API/services | jq
curl $API/assignments | jq
```

Required:

* Sprint 1 registration working
* Sprint 2 assignments working
* Service creation API working
* Logs visible for both processes

## Recommended terminal layout

Terminal 1:
Controller

Terminal 2:
Agent

Terminal 3:
API calls (curl)

Terminal 4 (optional):
Log observation

---

# Test 1 — Agent steady state with no assignments

## Steps

1 Start controller
2 Start agent
3 Do NOT create any services
4 Wait ~10 seconds

Terminal 1:
```bash
cargo run -p homeedge-controller
```

Terminal 2:
```bash
cargo run -p homeedge-agent
```

Terminal 3:
Verify no assignments:
```bash
curl $API/assignments/$NODE | jq
```

Verify node health:
```bash
curl $API/nodes | jq
```

Wait ~10 seconds.

## Expected result

Agent:

* Registers successfully
* Sends heartbeats
* Starts no services

Controller:

* Receives heartbeat
* Reports empty services list

## Expected logs

Agent:

```
registered node ...
heartbeat sent for node ...
```

Controller:

```
heartbeat received node_id=... services=[]
```

PASS if:

* No services started
* Heartbeat continues normally

---

# Test 2 — Assigned service starts automatically

## Steps

Terminal 3:
Create service:

```bash
curl -X POST $API/services \
-H "content-type: application/json" \
-d '{
"name":"lighting",
"version":"v2"
}'
```

Save returned id:
```bash
export SVC=<service_id>
```

Verify service exists:
```bash
curl $API/services | jq
```

Assign service:
```bash
curl -X POST $API/assignments \
-H "content-type: application/json" \
-d "{
\"service_id\":\"$SVC\",
\"node_id\":\"$NODE\"
}"
```

Verify assignment:
```bash
curl $API/assignments | jq
```

Wait ~5 seconds.
Verify runtime.
```bash
curl $API/nodes | jq
```

Wait one reconcile interval (~5s).

## Expected result

Agent:

* Detects new assignment
* Computes reconcile diff
* Starts worker task

Controller:

* Receives heartbeat showing Running service

## Expected logs

Agent:

```
reconciliation diff computed start_count=1 stop_count=0

starting service service_id=...
service started service_id=...

service worker started service_id=...
```

Controller:

```
heartbeat received node_id=...
services=[lighting-v2: Running]
```

PASS if:

* Exactly one start occurs
* Worker heartbeat logs appear
* Controller sees Running state

---

# Test 3 — Reconcile idempotency

## Steps

Leave assignment unchanged.
Wait multiple reconcile intervals.

Terminal 3:
Do nothing.
Wait 15–20 seconds.
Verify no duplicates:
```bash
curl $API/nodes | jq
```

## Expected result

Agent must NOT:

* Restart service
* Duplicate runtime entries
* Spawn extra workers

PASS if:

* Only one "service started" log exists
* Heartbeat stable
* No duplicate workers

---

# Test 4 — Removing assignment stops service

## Steps

Remove assignment from controller.
Wait one reconcile interval.
Terminal 3:
Remove assignment:

```bash
curl -X DELETE $API/assignments/service/$SVC
```

Verify removed:
```bash
curl $API/assignments | jq
```

Wait ~5 seconds.
Verify runtime stopped:
```bash
curl $API/nodes | jq
```

## Expected result

Agent:

* Computes stop diff
* Aborts worker
* Removes instance

Controller:

* Heartbeat shows empty services list

## Expected logs

Agent:

```
reconciliation diff computed start_count=0 stop_count=1

stopping service service_id=...
service stopped service_id=...
```

Controller:

```
heartbeat received node_id=... services=[]
```

PASS if:

* Service stops cleanly
* No crashes
* Heartbeat reflects removal

---

# Test 5 — Reassigning service restarts it

## Steps

Reassign the same service.
Wait one reconcile interval.

Reassign service:

```bash
curl -X PUT $API/assignments/service/$SVC \
-H "content-type: application/json" \
-d "{
\"node_id\":\"$NODE\"
}"
```

Wait ~5 seconds.
Verify running again:
```bash
curl $API/nodes | jq
```

## Expected result

Agent:

* Starts service again

Controller:

* Reports Running again

PASS if:

* Restart works cleanly
* No stale runtime state remains

---

# Test 6 — Multiple services reconcile correctly

## Steps

Create two services:
* lighting-v2
* hvac-v1

Assign both.
Wait reconcile.
Remove only one assignment.
Wait reconcile.

Create second service:
```bash
curl -X POST $API/services \
-H "content-type: application/json" \
-d '{
"name":"hvac",
"version":"v1"
}'
```

Save id:
```bash
export SVC2=<hvac_service_id>
```

Assign both:
```bash
curl -X POST $API/assignments \
-H "content-type: application/json" \
-d "{
\"service_id\":\"$SVC2\",
\"node_id\":\"$NODE\"
}"
```

Verify:
```bash
curl $API/assignments | jq
```

Wait 5 secs:
```bash
curl $API/nodes | jq
```

Remove one:
```bash
curl -X DELETE $API/assignments/service/$SVC
```

Wait ~5 seconds:
```bash
curl $API/nodes | jq
```




## Expected result

Agent:

* Starts both initially
* Stops only removed service

Controller:

* Shows both Running
* Then shows only remaining Running

PASS if:

* Selective reconciliation works
* No unrelated services affected

---


# Sprint 3 Acceptance Criteria

Sprint 3 is complete when:

✓ Agent polls desired assignments
✓ Agent computes reconcile diff
✓ Agent starts missing services
✓ Agent stops stale services
✓ Reconcile loop is idempotent
✓ Heartbeat includes service statuses
✓ Controller logs received statuses
✓ No crashes during steady state
✓ Restart converges correctly

---

# Quick Acceptance Run (5 minute validation)

Minimal demo sequence:

1 Start controller
2 Start agent
3 Verify empty heartbeat
4 Create service
5 Assign service
6 Verify service starts
7 Verify Running heartbeat
8 Remove assignment
9 Verify service stops
10 Verify empty heartbeat

If this passes, Sprint 3 is complete.

---

# Expected Demo Output

Agent:

```
service started service_id=... name=lighting-v2
```

Controller:

```
heartbeat received node_id=...
services=[lighting-v2: Running]
```

This demonstrates:
Desired → Reconcile → Runtime → Health reporting

Core orchestration loop is now functional.

---

# Sprint 3 Outcome

At this point the system behaves like a minimal orchestrator:

Controller defines desired state
Agent enforces desired state
Runtime converges automatically
Health is observable

This completes the core orchestration milestone.
