# Contributing to HomeEdge Orchestrator

Thank you for your interest in contributing to HomeEdge Orchestrator.

This project aims to demonstrate clean distributed systems design in Rust, with emphasis on correctness, clarity, and resilience. Contributions that improve reliability, readability, and architecture are especially welcome.

---

## Ways to Contribute

### Code Improvements

- Bug fixes
- Performance improvements
- Error handling improvements
- Better async patterns
- Safer state management

### Infrastructure Improvements

- Integration tests
- Observability improvements
- Docker improvements
- Logging improvements

### Documentation

- Architecture explanations
- Code comments
- Demo improvements
- API documentation

### Future Roadmap Items

Good candidate areas:

- SQLite persistence
- Metrics
- Capability selectors
- Scheduling improvements
- Worker runtime evolution

---

## Development Setup

### Prerequisites

**Required:**

- Rust stable
- Docker
- Docker Compose

**Recommended:**

- `cargo-watch`
- `jq`
- `clippy`

---

## Building

Build the entire workspace:

```bash
cargo build --workspace
```

Run checks:

```bash
cargo check --workspace
cargo clippy --workspace
cargo test --workspace
```

Format code:

```bash
cargo fmt
```

### Running Locally

Start the controller:

```bash
cargo run -p homeedge-controller
```

Start an agent:

```bash
cargo run -p homeedge-agent
```

### Running the Demo Environment

Start full simulation:

```bash
./scripts/dev-up
```

Reset environment:

```bash
./scripts/dev-reset
```

Simulate failure:

```bash
./scripts/demo-fail-node
```

Assign service:

```bash
./scripts/demo-assign-service
```

---

## Testing

Run all tests:

```bash
cargo test --workspace
```

Run controller tests:

```bash
cargo test -p homeedge-controller
```

Run integration tests:

```bash
cargo test -p homeedge-integration-tests
```

---

## Project Architecture Guidelines

When contributing, preserve the following architectural principles:

### 1 — Separation of Responsibilities

**Controller:** desired state, scheduling, cluster view.  
**Agent:** reconciliation, execution, reporting.

Avoid mixing responsibilities between the two.

### 2 — Reconciliation-Driven Design

Avoid event-driven coupling. Prefer:

```
poll → diff → act → report
```

This ensures system convergence after failures.

### 3 — Keep Loops Simple

Agent loops should remain independent, predictable, and resilient to failure. Avoid complex cross-loop communication.

### 4 — State Safety First

**Prefer:**
- Explicit state transitions
- Small mutation scopes
- Clear ownership

**Avoid:**
- Hidden shared state
- Implicit coupling
- Large locks

### 5 — MVP-First Complexity

This project intentionally avoids premature complexity. Do **not** introduce distributed consensus, complex schedulers, or unnecessary abstractions unless justified by a real need.

---

## Coding Guidelines

### General Rust Standards

Follow idiomatic Rust: explicit error types, minimal allocations, and clear naming. Prefer `Result<T, Error>` over panics.

### Error Handling

Use structured errors with [`thiserror`](https://docs.rs/thiserror). Avoid `unwrap()` and `expect()` outside of tests.

### Logging

Use [`tracing`](https://docs.rs/tracing). Log state transitions, failures, and reconciliation actions. Avoid noisy logs inside tight loops.

### Async Guidelines

**Prefer:** Tokio tasks, explicit intervals, bounded retries.  
**Avoid:** blocking calls, hidden sleeps, uncontrolled spawning.

---

## Pull Request Process

### Before Submitting

Ensure the following all pass:

```bash
cargo fmt
cargo clippy
cargo test
```

PRs failing CI checks will not be merged.

### PR Guidelines

Good PRs are small, focused, and well explained. Avoid large refactors without prior discussion, unrelated changes, or formatting-only PRs.

### PR Description

Your description should explain the problem, approach, tradeoffs, and testing performed. Example:

```
Problem:
Agent reconcile loop could silently fail.

Solution:
Propagate errors and log structured failures.

Testing:
Integration test + manual Docker test.
```

### Commit Style

```
agent: improve reconcile error handling
controller: fix node offline transition
tests: add failover scenario
docs: update architecture section
```

Format: `component: short description`

---

## Design Discussions

For major changes — such as scheduler redesigns, storage changes, runtime changes, or API changes — open an issue first. This avoids wasted work.

---

## What Not to Contribute (For Now)

To keep scope focused, please avoid:

- UI dashboards
- Authentication layers
- Distributed consensus
- Production security layers
- Complex orchestration features

These are outside the current MVP scope.

---

## Contribution License

By submitting a contribution you agree that your contribution will be licensed under the project's Business Source License (BSL 1.1).

---

## Code of Conduct

Be constructive and technical in discussions. Focus on correctness, clarity, and engineering tradeoffs.

---

## Questions

Open a GitHub issue for design questions, architecture discussion, or roadmap ideas.

---

## Thank You

Contributions that improve clarity, correctness, and resilience are especially valuable. This project aims to stay small, understandable, and technically clean.
