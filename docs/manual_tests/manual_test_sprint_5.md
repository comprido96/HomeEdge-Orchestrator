# Sprint 5 Manual Test -- Service Reassignment Failover

This test demonstrates automatic service reassignment when a node goes
offline.

## Goal

Verify:

1.  Services are assigned to nodes
2.  A node is killed
3.  Controller marks node Offline
4.  Services are reassigned
5.  Receiving agent starts the reassigned service

------------------------------------------------------------------------

## Terminal layout

Use 4 terminals:

Terminal 1 → Controller\
Terminal 2 → Agent‑1\
Terminal 3 → Agent‑2\
Terminal 4 → Agent‑3

------------------------------------------------------------------------

## 1 --- Start controller

Terminal 1:

``` bash
cargo run -p homeedge-controller
```

Expected: Controller starts API server.

------------------------------------------------------------------------

## 2 --- Start three agents

Terminal 2:

``` bash
cargo run -p homeedge-agent
```

Terminal 3:

``` bash
cargo run -p homeedge-agent
```

Terminal 4:

``` bash
cargo run -p homeedge-agent
```

Wait \~10 seconds for heartbeats.

------------------------------------------------------------------------

## 3 --- Verify nodes registered

Terminal 5 (or reuse one):

``` bash
API=http://127.0.0.1:8080

curl $API/nodes | jq
```

Expected: 3 healthy nodes.

------------------------------------------------------------------------

## 4 --- Register a service

``` bash
curl -X POST $API/services   -H "Content-Type: application/json"   -d '{
    "name":"lighting",
    "version":"v2"
  }'
```

------------------------------------------------------------------------

## 5 --- Assign service

Copy a node_id from previous step.

``` bash
NODE=<node_id>

curl -X POST $API/assignments/$NODE   -H "Content-Type: application/json"   -d '{
    "service_name":"lighting",
    "service_version":"v2"
  }'
```

Verify:

``` bash
curl $API/assignments | jq
```

Wait \~5 seconds.

Expected: One agent logs:

    INFO service started service_id=lighting-v2

------------------------------------------------------------------------

## 6 --- Kill one agent

Kill Agent‑2 (Terminal 3):

CTRL+C

Wait \~15--30 seconds.

------------------------------------------------------------------------

## 7 --- Observe controller logs

Expected:

    WARN node marked offline
    INFO service reassigned

------------------------------------------------------------------------

## 8 --- Observe receiving agent

One of the remaining agents should log:

    INFO service started service_id=lighting-v2

------------------------------------------------------------------------

## Expected demo flow

    Start 3 agents. Assign services. Kill agent‑2.

    [controller] WARN  node marked offline
    [controller] INFO  service reassigned
    [agent]      INFO  service started

------------------------------------------------------------------------

## Success criteria

Test passes if:

• Offline node detected\
• Service removed from dead node\
• Service reassigned\
• Another agent starts it

End of test.
