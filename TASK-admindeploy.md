You are in the DOKIPON repo, git worktree at .worktrees/wt-admindeploy (branch agent/codex/admindeploy).

# Task — k3s deployment for the admin service, and an end-to-end test runner that actually runs

Two deliverables. Both are needed for "start it and see the test results".

## Part 1 — k3s manifests for the admin service

`services/deploy/k3s/` already exists and deploys the 7 commerce services:

- `00-namespace-configmap.yaml` — namespace `commerce` + ConfigMap `commerce-config`
- `10-services.yaml` — 7 Deployment/Service pairs. Only `commerce-bff` is exposed
  (NodePort 30080); the rest are ClusterIP. Ports 8080-8086.
- `build-images.sh` — builds one image per service from the VM-local
  `cargo build --release` output (`ubuntu:26.04` base, image name `dokipon/<svc>:0.1.0`)
  and imports it into k3s containerd under `docker.io/dokipon/...`.

Read all three before writing anything. Your changes must be consistent with them.

Add the admin service (port **8087**, per the other agent's task):

1. In `10-services.yaml`, append a Deployment + Service for `commerce-admin` on 8087.
   Match the existing pattern exactly: same labels, same `envFrom: commerce-config`,
   same `PORT` env, same resources, same securityContext.
   **The probe method matters.** `commerce-identity` and `commerce-payment-adapter`
   expose `POST /healthz` and therefore use `tcpSocket` probes; the rest use `httpGet`.
   Pick the one matching the admin service's actual `/healthz` handler — you must READ
   the admin service source to decide. If the admin service does not exist yet in your
   worktree (another agent is writing it in parallel), default to `tcpSocket` and say so
   in your report; that choice cannot produce a false-positive readiness.
2. Expose the admin service so the console and the test runner can reach it. Pick a
   NodePort that does not collide with 30080. Document the choice.
3. Add `ADMIN_USER` / `ADMIN_PASSWORD` / and the upstream `*_URL` values the admin
   service reads. **Do not put a real password in a tracked file.** Use a
   `Secret` with obviously-placeholder values, or read from an existing Secret name and
   document that it must be created out-of-band. State your choice.
4. Update `build-images.sh`'s `ALL_SERVICES` array to include `commerce-admin`.

## Part 2 — an e2e test runner that can actually execute

`tools/e2e_checkout.sh` exists: 22 assertions over the checkout saga, targeting the
**order service on 8085**. Read it in full.

Two known blockers make it unrunnable outside the original author's Windows box:

1. Line 10: `TMPDIR_E2E="C:/Users/leo19/AppData/Local/Temp/gemarc-e2e"` — a hardcoded
   Windows path. On Linux `mkdir -p` creates a literal directory named `C:` in the cwd.
2. `ORDER="http://127.0.0.1:8085"` is hardcoded — but in k3s the order service is a
   ClusterIP that is not on the host loopback.

Produce `tools/e2e_checkout_k3s.sh`:

- **Do not modify `tools/e2e_checkout.sh`.** It is a tracked artifact with a deliberate
  design (the comment on line 14 explains why the Windows path is absolute: native
  Windows `curl` and MSYS `cat` resolve `/tmp` differently). Your script is the Linux /
  k3s variant.
- `TMPDIR_E2E` must come from `${TMPDIR:-/tmp}/gemarc-e2e` and be created with `mkdir -p`.
- `ORDER` must come from `${ORDER_URL:-http://commerce-order.rust-game-server.svc:5432}`-
  style configuration — actually: make it `${ORDER_URL:-http://commerce-order:8085}` and
  document that the default only resolves from inside the cluster, so the script's
  documented usage is `kubectl run` / `kubectl exec` into a pod in that namespace, OR a
  `kubectl port-forward`. Pick ONE and make the script's own header say exactly how to
  run it, with the command.
- Keep every assertion identical. Same test ids, same expected values. This is a port,
  not a rewrite. A "fixed" runner that quietly weakened an assertion is worse than none.
- Add `--verbose` to print the raw response for any FAIL, so a failure is diagnosable
  without re-running under a debugger.
- Ensure it exits non-zero on failure and zero on success, and that the summary line
  reports the true pass/fail counts.

Additionally produce `tools/e2e_admin_smoke.sh`: a smaller runner for the admin surface
— login as operator, list orders, fetch one order detail, and assert that reconcile on a
non-`payment_unknown` order is refused with 409. This proves the role matrix and the
recovery guard actually work end to end. Configurable `ADMIN_URL`, default
`http://localhost:30087` (your chosen NodePort) with a note on how to port-forward.

## Constraints

- Do NOT run kubectl, docker, or touch any cluster. Authoring only.
- Do NOT modify `tools/e2e_checkout.sh`.
- Do NOT touch the Rust or Dart sources.
- Do NOT touch `contracts/openapi.yaml` or the top-level README/CHANGELOG/CLAUDE/AGENTS
  (another agent owns those).
- Bash: `set -euo pipefail`, `shellcheck`-clean if shellcheck is available (run it and
  report), POSIX-ish but bash arrays are fine. Comments in English to match the repo.
- Every bash script must be syntactically valid: verify with `bash -n`.

## Definition of done

- `services/deploy/k3s/` updated for commerce-admin (manifest + build-images.sh)
- `tools/e2e_checkout_k3s.sh` and `tools/e2e_admin_smoke.sh` created
- `bash -n` clean on both new scripts; assertions byte-identical to the original
- Report: the NodePort you chose and why, the probe method you chose and the evidence
  for it, how each script is meant to be invoked, and what you could NOT verify without a
  cluster.
