# Aaroneous Troubleshooting Guide

A reference for resolving common runtime errors. The format
is **symptom → likely cause → fix → what to read next**.

If the answer is not here, file an issue with:

* The exact error message (verbatim, including any
  JSON body for HTTP failures).
* The commit hash (`aaroneous --version` or
  `GET /version`).
* A short log capture from
  `AARONEOUS_LOG=debug aaroneous`.

---

## Startup

### Symptom: "port already in use"

**Likely cause:** Another process is bound to the same
address, or a previous Aaroneous instance is still draining.

**Fix:**

1. `lsof -i :8080` (Linux/macOS) or
   `netstat -ano | findstr :8080` (Windows) to find the
   holder.
2. Either kill the conflicting process or set
   `AARONEOUS_BIND=127.0.0.1:9090` (or any other free
   address).

### Symptom: "api key required" returned from every call

**Likely cause:** `AARONEOUS_API_KEY` is unset on the
server. The `/healthz`, `/readyz`, `/live`, `/metrics`,
`/version`, and `/v1/models` routes are always open; all
other routes are gated.

**Fix:**

* Set `AARONEOUS_API_KEY=...` on the server. Clients send
  `Authorization: Bearer <key>`.
* Or, run behind a reverse proxy that injects the header
  after authenticating the client.

### Symptom: "genome parse error: ..."

**Likely cause:** A `.genome` file in `$AARONEOUS_DATA/genomes`
is corrupt or in a newer format than this build supports.

**Fix:**

1. Identify the file: the error message includes the path.
2. Re-download or revert the genome to a known-good version.
3. If you authored the genome locally, run
   `aaroneous --validate-genome path/to/genome` to get a
   line-level error.

---

## Runtime

### Symptom: tick watchdog warnings in the log

**Log line:** `WARN aaroneous::autonomic_loop: tick exceeded
watchdog budget (took Xs, budget Ys)`.

**Likely cause:** A tick is taking longer than
`AARONEOUS_TICK_WATCHDOG` (default 10s). This is normal
during cold start (JIT warming, genome load). Under load
it indicates a slow adapter.

**Fix:**

* First occurrence: ignore, observe the next few ticks.
* Persistent: profile with `cargo flamegraph --bin aaroneous`.
* If a single specialist is the slow path, raise
  `AARONEOUS_TICK_WATCHDOG` while you investigate.

### Symptom: tick budget exhausted

**Log line:** `ERROR aaroneous::autonomic_loop: tick budget
exhausted; shutting down cooperative loop`.

**Likely cause:** The loop has run for `AARONEOUS_MAX_TICKS`
(default 86,400) ticks. This is a self-imposed cap to
guard against runaway loops.

**Fix:**

* Raise `AARONEOUS_MAX_TICKS` to confirm it is the cap
  and not a real loop bug.
* If ticks fire faster than once a second, the cap will
  be hit within a day. Either lower the tick rate or
  raise the cap.

### Symptom: 503 from `/readyz`

**Likely cause:** One or more required adapters have not
reported in. The process is alive (`/healthz` returns 200)
but is not ready to serve traffic.

**Fix:**

1. `GET /status` (with API key) to see which subsystem
   is "not_ready".
2. Common offenders: the wasmtime engine on first start
   (JIT warming), or a federation link that has lost
   contact with a peer.
3. `GET /status/<subsystem>` to drill in.

### Symptom: 429 from every request

**Likely cause:** Rate limiting is enabled and the client
has exhausted its token bucket. The response includes a
`Retry-After` header.

**Fix:**

* Wait for `Retry-After` seconds and retry with
  exponential backoff.
* If the limit is too tight, raise
  `AARONEOUS_RATE_BURST` and `AARONEOUS_RATE_REFILL`
  (see the rate-limiter module).
* If the client is multi-tenant, the per-key bucket is
  keyed on the auth subject; a noisy neighbour can be
  quarantined by giving it a dedicated key.

### Symptom: 401 from `/v1/chat/completions`

**Likely cause:** Missing or wrong `Authorization` header.

**Fix:**

* Send `Authorization: Bearer <AARONEOUS_API_KEY>`.
* The key is set on the *server*. The client does not
  have its own key; the same server-side key authorises
  every client.

---

## Performance

### Symptom: high CPU with low request rate

**Likely cause:** A specialist is being invoked in a tight
loop (e.g. an agent is reading its own output).

**Fix:**

* Check the metrics endpoint:
  `curl http://localhost:8080/metrics | grep specialist`.
* Look for one specialist whose `invocations_total` is
  far higher than the others.
* Disable that specialist with
  `POST /v1/specialists/<name>/disable` (API key
  required).

### Symptom: memory grows unbounded over hours

**Likely cause:** A specialist is leaking wasm instances
or accumulating state.

**Fix:**

* Restart the process; if memory is fine after the
  restart, this is a leak.
* File an issue with the genome hash and a heap snapshot
  (`aaroneous --dump-heap`) if you can reproduce.
* As a temporary mitigation, schedule a daily restart.

---

## Tests

### Symptom: 13 tests fail in `cargo test -p a_run --lib`

**Likely cause:** Pre-existing. These failures are
documented in
`docs/maintenance/test-failures-2026-06-06.md`.

**Fix:**

* Do not block on them. The failures are in test logic
  (assertions on reversed iterators, missing
  preconditions) rather than runtime state.
* Pick the one you want to address, follow the
  recommended fix in the document, and submit a PR.

### Symptom: build errors after a `git pull`

**Likely cause:** A breaking change in a transitive
dependency, or a new MSRV requirement.

**Fix:**

1. `rustc --version`. The minimum supported is 1.85.
2. `cargo update -p <crate>` to roll the suspect dep
   forward.
3. If the build is still broken, check
   `Cargo.lock` for a yanked crate and pin the previous
   version.

---

## Getting More Help

* `INDEX.md` for the documentation index.
* `AGENTS.md` for the project workflow and the agent
  roles you can summon.
* `docs/api.md` for endpoint reference.
* `docs/deployment.md` for the deployment runbook.
