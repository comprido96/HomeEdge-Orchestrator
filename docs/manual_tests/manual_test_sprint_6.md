# Sprint 6 Manual Test -- Demo Infrastructure

This manual test validates the Docker demo environment, node failure
handling, and automatic service reassignment.

## Prerequisites

-   Docker installed
-   Docker Compose available
-   Run from repository root

## 1. Start the full system

Build and start everything:

``` bash
docker compose up --build
```

Expected result:

-   Controller starts on port 8080
-   Three agents start
-   Agents register automatically
-   Heartbeats begin flowing

Typical log indicators:

    node registered
    heartbeat received
    agent starting

Leave this running.

## 2. Verify agents are running (optional)

In another terminal:

``` bash
docker compose ps
```

Expected:

    controller   running
    agent-1      running
    agent-2      running
    agent-3      running

## 3. Simulate node failure

Stop one agent:

``` bash
./scripts/demo-fail-node.sh
```

(or)

``` bash
docker compose stop agent-2
```

## 4. Observe failover behaviour

Watch controller logs.

Expected sequence:

    node marked offline
    service reassigned

Agent receiving work should show:

    service started

System should stabilise within \~30--40 seconds (depending on stale node
timeout).

## 5. Reset environment

Stop everything:

``` bash
./scripts/dev-reset.sh
```

(or manually)

``` bash
docker compose down -v
```

Expected:

-   All containers removed
-   Network removed
-   Volumes removed

## Success Criteria

Sprint 6 is successful if:

-   System starts with one command
-   All agents register automatically
-   Killing one agent does not crash system
-   Services reassign automatically
-   Remaining agents continue heartbeats
