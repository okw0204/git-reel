# Local Dev Makefile Implementation Plan

> **For agentic workers:** REQUIRED SUB-SKILL: Use superpowers:subagent-driven-development (recommended) or superpowers:executing-plans to implement this plan task-by-task. Steps use checkbox (`- [ ]`) syntax for tracking.

**Goal:** Add a one-command local development startup path with `make dev`.

**Architecture:** Keep the existing Rust API and Vite web dev servers unchanged. Add a root `Makefile` that starts both long-running processes and updates `README.md` so `make dev` is the primary local startup path.

**Tech Stack:** GNU Make, npm workspaces, Vite, Rust/Cargo, Axum.

---

## File Structure

- Create: `Makefile` - root developer shortcuts for local startup and tests.
- Modify: `README.md` - document `make dev` as the default local startup command and keep manual split-terminal commands as a fallback.

### Task 1: Add `make dev`

**Files:**
- Create: `Makefile`

- [ ] **Step 1: Add the Makefile**

```make
.PHONY: dev test test-web test-server

dev:
	@trap 'kill 0' INT TERM EXIT; \
	cargo run --manifest-path server/Cargo.toml & \
	npm run dev:web & \
	wait

test:
	npm test

test-web:
	npm run test:web

test-server:
	npm run test:server
```

- [ ] **Step 2: Verify the recipe expands**

Run: `make -n dev`

Expected: the output shows a shell recipe that starts `cargo run --manifest-path server/Cargo.toml` and `npm run dev:web`.

### Task 2: Document local startup

**Files:**
- Modify: `README.md`

- [ ] **Step 1: Update the development server section**

Replace the current two-terminal-first instructions with `make dev` as the primary command. Keep the two separate commands as fallback instructions for debugging each side independently.

- [ ] **Step 2: Verify docs mention both paths**

Run: inspect `README.md`.

Expected: `make dev`, `cargo run --manifest-path server/Cargo.toml`, and `npm run dev:web` are all documented.

### Task 3: Final Verification

**Files:**
- Existing tests only.

- [ ] **Step 1: Run existing tests**

Run: `npm test`

Expected: frontend and server tests pass.

## Self-Review

- Spec coverage: covers one-command startup and README documentation.
- Placeholder scan: no placeholders remain.
- Type consistency: no new types or APIs are introduced.
