# Sprint 2 Manual Test Checklist

## Objective
Verify the Sprint 2 control-plane flow end to end:

- agent registers successfully
- agent becomes healthy via heartbeat
- controller accepts service definitions through `POST /services`
- controller lists services through `GET /services`
- controller assigns unassigned services to the first healthy node
- `GET /assignments/{node_id}` returns real `Vec<ServiceAssignment>` data
- agent polls assignments and stores desired state without starting services

## Preconditions

- workspace builds successfully
- controller and agent binaries compile
- no stale controller or agent processes are running
- default controller address is `http://127.0.0.1:8080`

## Terminal Setup

Use two terminals.

### Terminal 1: start controller

```bash
cargo run -p homeedge-controller
```

Expected log contains something like:

```text
homeedge-controller listening addr=127.0.0.1:8080
```

### Terminal 2: start agent

```bash
cargo run -p homeedge-agent
```

Expected log sequence includes:

```text
homeedge-agent starting
registered node <node-id> with controller
assignments unchanged for node <node-id>: count=0
```

Heartbeat logs may also appear depending on current logging.

## Test 1 — Node registration and health

### Step
List nodes from the controller:

```bash
curl http://127.0.0.1:8080/nodes
```

### Expected

- response is `200 OK`
- response includes exactly one node after one agent is started
- node has a valid UUID
- node status becomes `Healthy`

### Record

Node ID under test:

```text
____________________________
```

## Test 2 — No assignments before services exist

### Step
Fetch assignments for the node ID recorded above:

```bash
curl http://127.0.0.1:8080/assignments/<node-id>
```

### Expected

- response is `200 OK`
- response body is an empty JSON array:

```json
[]
```

- agent log shows assignment count `0`

## Test 3 — Create a service definition

### Step
Create a service:

```bash
curl -X POST http://127.0.0.1:8080/services \
  -H 'content-type: application/json' \
  -d '{"name":"lighting","version":"v1","selector":null}'
```

### Expected

- response is `201 Created`
- response body contains a generated service ID
- returned service fields match:
  - `name = lighting`
  - `version = v1`
  - `selector = null`

Example shape:

```json
{"service":{"id":"<service-id>","name":"lighting","version":"v1","selector":null}}
```

### Record

Service ID under test:

```text
____________________________
```

## Test 4 — List services

### Step
List services:

```bash
curl http://127.0.0.1:8080/services
```

### Expected

- response is `200 OK`
- response contains the newly created service
- service ID matches the ID from Test 3

## Test 5 — Duplicate service rejection

### Step
Submit the same service again:

```bash
curl -X POST http://127.0.0.1:8080/services \
  -H 'content-type: application/json' \
  -d '{"name":"lighting","version":"v1","selector":null}'
```

### Expected

- response is `409 Conflict`
- response body contains an error message describing the duplicate

Example shape:

```json
{"error":"conflict: service 'lighting' version 'v1' already exists"}
```

## Test 6 — Assignment materialization on controller

### Step
Fetch assignments for the node again:

```bash
curl http://127.0.0.1:8080/assignments/<node-id>
```

### Expected

- response is `200 OK`
- response body is a non-empty JSON array
- array contains one `ServiceAssignment`
- `service_id` matches the service created in Test 3
- `node_id` matches the node from Test 1

Example shape:

```json
[
  {
    "service_id": "<service-id>",
    "node_id": "<node-id>"
  }
]
```

## Test 7 — Unknown node returns not found

### Step
Call assignments with a random UUID not present in controller state:

```bash
curl http://127.0.0.1:8080/assignments/00000000-0000-0000-0000-000000000001
```

### Expected

- response is `404 Not Found`
- body contains:

```json
{"error":"node not found"}
```

## Test 8 — Agent observes assignment changes

### Step
Watch the agent log after Test 3.

### Expected

Within one polling interval, the agent log changes from zero assignments to one assignment.

Expected pattern:

```text
assignments updated for node <node-id>: count=1
```

Later polls with the same assignment set should log:

```text
assignments unchanged for node <node-id>: count=1
```

## Test 9 — Agent does not start services yet

### Step
Observe agent logs for at least two polling intervals after assignment appears.

### Expected

- agent stores desired assignments only
- no local service runtime or worker start logs appear
- no service start/reconcile behavior is present yet

This is the correct Sprint 2 behavior.

## Optional Test 10 — Multiple services map to the first healthy node

### Step
Create a second service:

```bash
curl -X POST http://127.0.0.1:8080/services \
  -H 'content-type: application/json' \
  -d '{"name":"hvac","version":"v1","selector":null}'
```

Then fetch assignments:

```bash
curl http://127.0.0.1:8080/assignments/<node-id>
```

### Expected

- response contains both service assignments
- both are assigned to the same node in a single-agent setup
- this matches first-fit MVP scheduling

## Pass Criteria

Sprint 2 manual validation passes if all of the following are true:

- controller accepts and lists services
- duplicate service creation is rejected with `409 Conflict`
- assignments endpoint returns real data, not dummy payloads
- assignments reference the correct `node_id` and `service_id`
- agent observes assignment changes and stores desired state
- agent does not start services yet

## Notes

- `GET /assignments/{node_id}` requires a node ID, not a service ID
- assignments only appear after at least one healthy agent has registered
- in a single-agent environment, first-fit scheduling places all services on that one healthy node
- service execution belongs to Sprint 3,