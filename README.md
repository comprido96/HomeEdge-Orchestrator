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
┌─────────────────────────────────────────────────┐
│                   VPS (Controller)              │
│                                                 │
│   POST /register       ┌──────────────────┐    │
│   POST /heartbeat  ──▶ │  In-Memory State │    │
│   GET  /assignments    │  Node Registry   │    │
│   GET  /nodes          │  Service Assign. │    │
│                        └──────────────────┘    │
│              Failure Detection & Reassignment   │
└────────────────────────┬────────────────────────┘
                         │ VPN (Headscale)
          ┌──────────────┼──────────────┐
          │              │              │
   ┌──────▼──────┐ ┌─────▼──────┐ ┌───▼────────┐
   │  Node Agent │ │ Node Agent │ │ Node Agent │
   │   (edge-1)  │ │  (edge-2)  │ │  (edge-3)  │
   └─────────────┘ └────────────┘ └────────────┘
```

---

## Features

- **Node Registration** — Agents self-register with the controller on startup
- **Heartbeat Monitoring** — Periodic liveness signals with configurable timeout detection
- **Service Assignment** — Controller distributes named, versioned services to nodes based on selectors
- **Failure Detection** — Stale/offline nodes are automatically identified
- **Automatic Reassignment** — Services are redistributed to healthy nodes on failure
- **Service Lifecycle Simulation** — Agents track service states: `assigned → starting → running → failed`
- **Async Throughout** — Built on Tokio; non-blocking I/O across all components

---

## Tech Stack

- **[Rust](https://www.rust-lang.org/)** — Systems language powering both components
- **[Tokio](https://tokio.rs/)** — Async runtime
- **[Axum](https://github.com/tokio-rs/axum)** — HTTP server framework for the controller
- **[Reqwest](https://github.com/seanmonstar/reqwest)** — Async HTTP client for node agents
- **[Docker](https://www.docker.com/)** — Multi-node simulation environment

---

## Project Structure

```
homeedge-orchestrator/
├── Cargo.toml              # Workspace manifest
├── controller/
│   ├── Cargo.toml
│   └── src/
│       ├── main.rs
│       ├── routes/         # API endpoint handlers
│       ├── state/          # Node registry and service assignments
│       └── scheduler/      # Failure detection and reassignment logic
├── agent/
│   ├── Cargo.toml
│   └── src/
│       ├── main.rs
│       ├── client/         # Controller API client
│       └── runtime/        # Local service lifecycle management
└── docker/
    ├── docker-compose.yml
    ├── Dockerfile.controller
    └── Dockerfile.agent
```

---

## Getting Started

### Prerequisites

- Rust (stable) — [Install via rustup](https://rustup.rs/)
- Docker & Docker Compose

### Build

```bash
git clone https://github.com/your-username/homeedge-orchestrator
cd homeedge-orchestrator
cargo build --workspace
```

### Run Locally (without Docker)

Start the controller:
```bash
cargo run -p controller
```

Start a node agent in a separate terminal:
```bash
NODE_ID=edge-1 cargo run -p agent
```

### Run the Full Demo (Docker)

```bash
docker compose -f docker/docker-compose.yml up
```

This spins up one controller and three node agents. Watch the logs to observe registration, assignment, and heartbeat activity.

---

## API Reference

### Controller Endpoints

| Method | Path | Description |
|--------|------|-------------|
| `POST` | `/register` | Register a new node agent |
| `POST` | `/heartbeat` | Receive a heartbeat from a node |
| `GET` | `/assignments/{node_id}` | Fetch service assignments for a node |
| `GET` | `/nodes` | List all known nodes and their status |

---

## Demo Scenario

The final demo runs a Docker environment with **one controller** and **three node agents** and demonstrates the full orchestration lifecycle:

1. All three nodes register with the controller
2. Services are assigned to nodes
3. Heartbeat monitoring begins
4. One node container is terminated (simulated failure)
5. Controller detects the missed heartbeats and marks the node offline
6. Services from the failed node are automatically reassigned to healthy nodes

---

## Roadmap

- [x] Controller HTTP API (register, heartbeat, assignments, nodes)
- [x] Node agent (registration, heartbeats, assignment polling, reconciliation)
- [x] Service assignment model with health status tracking
- [x] Heartbeat timeout and failure detection
- [x] Automatic service reassignment
- [x] Multi-node Docker simulation
- [ ] SQLite persistence for controller state
- [ ] Structured tracing and metrics (OpenTelemetry / `tracing` crate)

---

## Key Concepts Demonstrated

- **Control plane / data plane separation** — Controller manages desired state; agents enforce it locally
- **State reconciliation** — Agents continuously compare desired vs. actual service state
- **Failure detection** — Heartbeat timeouts drive node health transitions
- **Async Rust** — Tokio tasks, channels, and timers coordinate concurrent workloads
- **Workspace architecture** — Shared types and clean separation between crates

---

## License

MIT
