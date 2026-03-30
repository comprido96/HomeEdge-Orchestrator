# HomeEdge Orchestrator

> A lightweight Rust control plane and agent runtime for distributed smart-home node management.

---

## Overview

**HomeEdge Orchestrator** is a minimal orchestration platform built in Rust for managing distributed smart-home nodes deployed in customer environments. Mini-PCs installed in users' homes run home automation workloads and connect securely to a central VPS via a Headscale-powered VPN. As deployments scale, HomeEdge handles node health monitoring, service configuration distribution, and coordinated service assignment — all with the performance and safety guarantees of Rust.

The system is composed of two binaries:

| Component | Role |
|---|---|
| **Controller** | Central service on a VPS — tracks node health, manages desired state, assigns services |
| **Node Agent** | Daemon on each edge device — registers, sends heartbeats, fetches assignments, reconciles state |

---

## Architecture

```
┌─────────────────────────────────────────────┐
│                  Controller                 │
│                                             │
│               REST API                      │
│               Node registry                 │
│               Assignment engine             │
│               Failure detection             │
│               Reassignment loop             │
│                                             │
└───────────────┬─────────────────────────────┘
                │ HTTP
        ┌───────┼────────┬────────┐
        │       │        │        │
   ┌────▼───┐ ┌─▼────┐ ┌─▼────┐ ┌─▼────┐
   │ Agent  │ │Agent │ │Agent │ │Agent │
   │ node-1 │ │node-2│ │node-3│ │ ...  │
   └────────┘ └──────┘ └──────┘ └──────┘
```

Each agent runs:
- Registration loop
- Heartbeat loop
- Reconciliation loop
- Service runtime (Tokio tasks)

---

## Features

### Implemented

- **Node registration** — Agents register on startup via `POST /register`
- **Heartbeat monitoring** — Agents send periodic health reports; controller detects stale nodes
- **Desired state reconciliation** — Agents continuously compare desired services (controller) vs. running services (local)
- **Service assignment** — Controller assigns services to healthy nodes; agents pick them up via polling
- **Failure detection** — Nodes marked offline after heartbeat timeout
- **Automatic reassignment** — Services automatically migrate from failed nodes
- **Service lifecycle simulation** — Services run as Tokio tasks inside the agent
- **Integration tests** — Controller API tests and end-to-end orchestration flow tests

### Deferred (post-demo scope)

- SQLite persistence
- Metrics
- Worker process runtime
- Capability selectors
- Advanced scheduling

---

## Tech Stack

| Tool | Purpose |
|------|---------|
| **Rust** | Core language |
| **Tokio** | Async runtime |
| **Axum** | Controller HTTP server |
| **Reqwest** | Agent HTTP client |
| **Serde** | Serialization |
| **Tracing** | Structured logging |
| **Docker** | Multi-node simulation |

---

## Project Structure

```
.
├── Cargo.toml
├── crates
│   ├── homeedge-agent
│   │   ├── loops/                  # registration, heartbeat, reconciliation
│   │   ├── runtime/                # service lifecycle simulation
│   │   ├── controller_client.rs
│   │   └── app_state.rs
│   │
│   ├── homeedge-controller
│   │   ├── handlers/               # REST endpoints
│   │   ├── domain/                 # assignment + node logic
│   │   ├── background/             # failure detection + reassignment
│   │   ├── repository/             # in-memory state
│   │   └── router.rs
│   │
│   ├── homeedge-types
│   │   ├── node.rs                 # Node domain types
│   │   ├── service.rs              # Service domain types
│   │   └── api.rs                  # API contracts
│   │
│   ├── homeedge-test-utils
│   │   └── test helpers
│   │
│   ├── homeedge-integration-tests
│   │   └── end-to-end tests
│   │
│   └── homeedge-worker             # reserved for future service runtime
│
├── docker-compose.yml
├── Dockerfile.controller
├── Dockerfile.agent
│
├── scripts
│   ├── dev-up                      # start demo environment
│   ├── dev-reset                   # clean environment
│   ├── demo-assign-service         # helper script
│   └── demo-fail-node              # failure simulation
│
└── docs
    └── demo-script.md
```

---

## System Flow

### Startup

1. Controller starts HTTP server
2. Agents start and register
3. Agents begin heartbeats
4. Agents start reconciliation loops

### Service Creation

1. Service created via API
2. Controller assigns to a node
3. Agent detects assignment
4. Agent spawns Tokio service task
5. Agent reports service health

### Failure Recovery

1. Agent stops heartbeating
2. Controller marks node offline
3. Services reassigned
4. New agent starts service
5. System stabilizes

---

## Getting Started

### Prerequisites

- Rust stable
- Docker
- Docker Compose
- `jq` (optional, for API inspection)

### Build

```bash
cargo build --workspace
```

### Run Locally (Development)

Start the controller:

```bash
cargo run -p homeedge-controller
```

Start an agent:

```bash
cargo run -p homeedge-agent
```

### Run the Full Demo

Recommended method:

```bash
./scripts/dev-up
```

Alternative:

```bash
docker compose up --build
```

This starts 1 controller and 3 agents.

---

## Demo Scenario

### Step 1 — Start System

```bash
./scripts/dev-up
```

Agents register and begin heartbeating. Verify:

```bash
curl http://127.0.0.1:8080/nodes | jq
```

### Step 2 — Create Service

```bash
./scripts/demo-assign-service
```

Controller assigns the service to a node. Agent detects the assignment and starts the service.

### Step 3 — Simulate Failure

```bash
./scripts/demo-fail-node
```

Controller detects failure after heartbeat timeout. Service is reassigned automatically.

### Step 4 — Observe Recovery

A healthy node starts the reassigned service and the system returns to stable state.

### Step 5 — Cleanup

```bash
./scripts/dev-reset
```

---

## API Reference

| Method | Path | Description |
|--------|------|-------------|
| `POST` | `/register` | Register node |
| `POST` | `/heartbeat` | Receive heartbeat |
| `GET` | `/assignments/{node_id}` | Get node assignments |
| `GET` | `/nodes` | List nodes |
| `POST` | `/services` | Create service |
| `GET` | `/services` | List services |

---

## Key Concepts

### Control Plane / Data Plane Separation

The controller defines desired state; agents enforce it.

### Reconciliation Loops

Agents continuously execute:

```
poll → diff → act → report
```

This guarantees convergence even after failures.

### Failure Detection

Nodes transition from `Healthy → Offline` based on heartbeat timeout.

### Automatic Failover

Service reassignment happens automatically when nodes fail.

### Async Rust Architecture

The project demonstrates Tokio tasks, async HTTP, concurrent loops, structured logging, and workspace modularity.

## License

This project is licensed under the [Business Source License 1.1](LICENSE.md) (BSL).

Free for:
- Personal use
- Research
- Internal company use

Commercial redistribution or SaaS requires a commercial license.

**Contact:** your@email

After 4 years, the code converts to [Apache 2.0](http://www.apache.org/licenses/LICENSE-2.0).
