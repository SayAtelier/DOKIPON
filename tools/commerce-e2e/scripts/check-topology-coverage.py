"""check-topology-coverage.py — every contract path must be served by BOTH
deployment topologies.

## Why this exists

The two topologies are supposed to be the same product with a different
shape: the same service code, the same contract, the same features. That
promise is easy to break in one place and invisible everywhere else.

It already broke once. The k3s manifest was missing the `commerce-content`
service entirely, and the shared ConfigMap had no `CONTENT_URL`, so
`GET /v1/content` in the cluster topology silently returned an empty
document — the storefront's collections, series, trends, topics, pages, FAQ,
banners and size guides were all empty, while the function topology served
them in full. Nothing failed. The site just looked emptier in one
deployment than in the other.

So this check reads three sources and compares them:

- **the contract** — `contracts/openapi.yaml`, the paths of record;
- **the path table** — `services/crates/commerce-core/src/routes.rs`, which
  says which service owns each path prefix;
- **the deployments** — the k3s manifests and the Vercel function layout.

and then, for every contract path, asks: is the owning service *deployed*
in this topology, is it *reachable* through this topology's routing, and
does it have the *environment* it needs? A path that fails any of the three
in either topology is a finding.

Run:

    python tools/commerce-e2e/scripts/check-topology-coverage.py
    python tools/commerce-e2e/scripts/check-topology-coverage.py <k3s manifests dir>

The argument points the check at a different set of k3s manifests, which is
how the check is tested against a known-bad set.

Exits 1 with the list of gaps when there are any. PyYAML required.
"""

from __future__ import annotations

import json
import re
import sys
from pathlib import Path

import yaml

ROOT = Path(__file__).resolve().parents[3]
CONTRACT = ROOT / "contracts" / "openapi.yaml"
PATH_TABLE = ROOT / "services" / "crates" / "commerce-core" / "src" / "routes.rs"
K3S = Path(sys.argv[1]).resolve() if len(sys.argv) > 1 else ROOT / "services" / "deploy" / "k3s"
ENVOY = ROOT / "docs" / "deploy" / "03-platform" / "envoy" / "10-commerce-gateway.yaml"
VERCEL_JSON = ROOT / "vercel.json"
VERCEL_SRC = ROOT / "api"
ENV_EXAMPLE = ROOT / "services" / "deploy" / "vercel" / "env.example"
SERVICE_CRATES = ROOT / "services" / "crates"
CLIENT = ROOT / "packages" / "api_client" / "lib" / "src"

# How each service is named and wired in each topology.
#
# `k3s`: the Deployment's name and the environment variable that points at
# it. `vercel`: the Function's file name and the same variable.
SERVICES = {
    "Bff": {"k3s": ("commerce-bff", None), "vercel": ("gateway", None)},
    "Catalog": {"k3s": ("commerce-catalog", "CATALOG_URL"), "vercel": ("catalog", "CATALOG_URL")},
    "Cart": {"k3s": ("commerce-cart", "CART_URL"), "vercel": ("cart", "CART_URL")},
    "Order": {"k3s": ("commerce-order", "ORDER_URL"), "vercel": ("order", "ORDER_URL")},
    "Content": {"k3s": ("commerce-content", "CONTENT_URL"), "vercel": ("content", "CONTENT_URL")},
    "Identity": {"k3s": ("commerce-identity", "IDENTITY_URL"), "vercel": ("identity", "IDENTITY_URL")},
    "Inventory": {"k3s": ("commerce-inventory", "INVENTORY_URL"), "vercel": ("inventory", "INVENTORY_URL")},
    "Payment": {"k3s": ("commerce-payment-adapter", "PAYMENT_ADAPTER_URL"), "vercel": ("payment", "PAYMENT_ADAPTER_URL")},
    "Admin": {"k3s": ("commerce-admin", None), "vercel": ("admin", None)},
}

# The prefixes the Envoy gateway routes to the BFF, which then forwards
# them. Everything else under `/v1/` and `/admin/v1/` reaches its owner
# through it.
ENVOY_TO_BFF = ("/v1/", "/admin/v1/")


def contract_paths() -> list[tuple[str, str]]:
    """Every (method, path) the contract of record declares."""
    document = yaml.safe_load(CONTRACT.read_text(encoding="utf-8"))
    out = []
    for path, operations in (document.get("paths") or {}).items():
        if not isinstance(operations, dict):
            continue
        for method in operations:
            if method.lower() in {"get", "post", "put", "patch", "delete"}:
                out.append((method.upper(), path))
    return sorted(out)


def path_table() -> list[tuple[str, str]]:
    """The path table, read from the Rust source that owns it."""
    text = PATH_TABLE.read_text(encoding="utf-8")
    table = re.search(r"PATH_TABLE[^=]*=\s*&\[(.*?)\];", text, re.S)
    if not table:
        raise SystemExit(
            f"could not read PATH_TABLE out of {PATH_TABLE} — the path table's shape changed"
        )
    entries = re.findall(r'\("([^"]+)",\s*ServiceId::(\w+)\)', table.group(1))
    if not entries:
        raise SystemExit(f"PATH_TABLE in {PATH_TABLE} parsed to nothing")
    return entries


def normalize(path: str) -> str:
    """A path with its parameter names erased.

    The contract writes its parameters in camelCase (`{orderId}`) and the
    services write theirs in snake_case (`{order_id}`). They are the same
    path, so the comparison must not care which spelling a path uses for
    the name of its parameter.
    """
    return re.sub(r"\{[^}]*\}", "{}", path)


def owner_of(path: str, table: list[tuple[str, str]]) -> str | None:
    """The service that owns `path`, by the longest matching prefix."""
    best = None
    for prefix, service in table:
        if path == prefix or path.startswith(prefix + "/"):
            if best is None or len(prefix) > len(best[0]):
                best = (prefix, service)
    return best[1] if best else None


def k3s_facts() -> tuple[set[str], dict[str, int], dict[str, str]]:
    """(deployed services, their ports, the shared ConfigMap)."""
    deployed: set[str] = set()
    ports: dict[str, int] = {}
    configmap: dict[str, str] = {}
    for manifest in sorted(K3S.glob("*.yaml")):
        for document in yaml.safe_load_all(manifest.read_text(encoding="utf-8")):
            if not isinstance(document, dict):
                continue
            kind = document.get("kind")
            name = (document.get("metadata") or {}).get("name")
            if kind == "ConfigMap" and name:
                configmap.update(document.get("data") or {})
            elif kind == "Service" and name:
                for port in document.get("spec", {}).get("ports", []):
                    deployed.add(name)
                    ports[name] = port.get("port")
            elif kind == "Deployment" and name:
                containers = (
                    document.get("spec", {})
                    .get("template", {})
                    .get("spec", {})
                    .get("containers", [])
                )
                for container in containers:
                    for port in container.get("ports", []):
                        ports[name] = port.get("containerPort")
    return deployed, ports, configmap


def envoy_prefixes() -> list[tuple[str, str]]:
    """The (path prefix, backend service) pairs the Envoy gateway routes."""
    out: list[tuple[str, str]] = []
    if not ENVOY.exists():
        return out
    for document in yaml.safe_load_all(ENVOY.read_text(encoding="utf-8")):
        if not isinstance(document, dict) or document.get("kind") != "HTTPRoute":
            continue
        for rule in document.get("spec", {}).get("rules", []):
            backends = [b.get("name") for b in rule.get("backendRefs", [])]
            matches = rule.get("matches") or []
            if not matches:
                out.append(("", backends[0] if backends else ""))
                continue
            for match in matches:
                value = (match.get("path") or {}).get("value", "")
                out.append((value, backends[0] if backends else ""))
    return out


def vercel_facts() -> tuple[set[str], dict[str, str], list[str]]:
    """(the functions, the documented environment variables, the rewrites)."""
    functions = {p.stem for p in VERCEL_SRC.rglob("*.rs")}
    variables = []
    for line in ENV_EXAMPLE.read_text(encoding="utf-8").splitlines():
        match = re.match(r"^([A-Z][A-Z0-9_]*)=", line)
        if match:
            variables.append(match.group(1))
    vercel_json = json.loads(VERCEL_JSON.read_text(encoding="utf-8"))
    rewrites = [r.get("source", "") for r in vercel_json.get("rewrites", [])]
    return functions, {v: v for v in variables}, rewrites


def service_routes() -> dict[str, list[str]]:
    """Every route each commerce service registers, read from its source."""
    routes: dict[str, list[str]] = {}
    for crate in sorted(SERVICE_CRATES.glob("commerce-*")):
        lib = crate / "src" / "lib.rs"
        if not lib.exists():
            continue
        text = lib.read_text(encoding="utf-8")
        found = re.findall(r'\.route\(\s*"(/[^"]*)"', text)
        if found:
            routes[crate.name] = sorted(set(found))
    return routes


def client_paths() -> list[str]:
    """Every path the Dart client asks for.

    The contract declares the paths the clients use, so a path the client
    calls but the contract does not declare is a gap in the contract.
    """
    found: set[str] = set()
    for source in sorted(CLIENT.rglob("*.dart")):
        text = source.read_text(encoding="utf-8")
        for match in re.findall(r"'(/(?:v1|admin/v1)/[^']*)'", text):
            # The client writes its parameters as `$name`; the contract
            # writes them as `{name}`.
            found.add(re.sub(r"\$(\w+)", r"{\1}", match))
    return sorted(found)


def public_origin_routes() -> list[str]:
    """The routes the public origin serves itself.

    The BFF and the admin facade are the public origin: the paths they serve
    directly are the contract's surface. A path one of them serves but the
    contract does not declare is a gap in the contract.
    """
    out: list[str] = []
    for crate in ("commerce-bff", "commerce-admin"):
        lib = SERVICE_CRATES / crate / "src" / "lib.rs"
        if not lib.exists():
            continue
        out.extend(re.findall(r'\.route\(\s*"(/[^"]*)"', lib.read_text(encoding="utf-8")))
    return sorted(set(out))


def main() -> int:
    table = path_table()
    declared = contract_paths()
    deployed, ports, configmap = k3s_facts()
    envoy = envoy_prefixes()
    functions, variables, rewrites = vercel_facts()
    routes = service_routes()
    declared_paths = {normalize(path) for _, path in declared}

    problems: list[str] = []
    rows: list[dict] = []

    # --- The contract must declare what the clients call, and what the
    # --- public origin serves. -------------------------------------------
    for path in client_paths():
        if normalize(path) not in declared_paths:
            problems.append(
                f"contract: the client calls {path}, and the contract does not declare it"
            )
    for path in public_origin_routes():
        if path.startswith("/healthz"):
            continue
        if normalize(path) not in declared_paths:
            problems.append(
                f"contract: the public origin serves {path}, and the contract does not declare it"
            )

    # --- The paths to check: the contract's, plus every route a service
    # --- registers that the path table gives an owner. --------------------
    check_set: dict[str, set[str]] = {}
    for method, path in declared:
        check_set.setdefault(normalize(path), set()).add(method)
    for crate, crate_routes in routes.items():
        for route in crate_routes:
            if route.startswith("/healthz"):
                continue
            if owner_of(route, table) is None:
                # Not a public path — a service-to-service route.
                continue
            check_set.setdefault(normalize(route), set()).add("*")

    for path in sorted(check_set):
        methods = check_set[path]
        owner = owner_of(path, table)
        method = " ".join(sorted(methods))
        row = {"method": method, "path": path, "owner": owner, "k3s": "?", "vercel": "?"}

        if owner is None:
            problems.append(f"{method} {path}: no service owns this path in the path table")
            rows.append(row)
            continue

        # --- k3s ---------------------------------------------------------
        name, variable = SERVICES[owner]["k3s"]
        k3s_ok = True
        if name not in deployed:
            problems.append(
                f"k3s: {method} {path} is owned by {owner}, and {name} is not deployed in the "
                f"cluster topology"
            )
            k3s_ok = False
        if variable:
            url = configmap.get(variable)
            if not url:
                problems.append(
                    f"k3s: {method} {path} is owned by {owner}, and the shared ConfigMap has no "
                    f"{variable} for it to be reached at"
                )
                k3s_ok = False
            else:
                host = url.split("//")[-1].split(":")[0]
                port = url.rsplit(":", 1)[-1]
                if host not in deployed:
                    problems.append(
                        f"k3s: {method} {path} is owned by {owner}, and {variable} points at "
                        f"{host}, which is not a deployed service"
                    )
                    k3s_ok = False
                elif str(ports.get(host)) != port:
                    problems.append(
                        f"k3s: {method} {path} is owned by {owner}, and {variable} points at "
                        f"port {port} while {host} serves {ports.get(host)}"
                    )
                    k3s_ok = False
        # Reachability: the Envoy gateway must send the path to the owner, or
        # to the BFF that forwards it.
        routed = False
        for prefix, backend in envoy:
            if not prefix:
                continue
            if path == prefix.rstrip("/") or path.startswith(prefix):
                routed = routed or backend == name or backend == "commerce-bff"
        if envoy and not routed:
            problems.append(
                f"k3s: {method} {path} is owned by {owner}, and no Envoy route reaches it"
            )
            k3s_ok = False
        row["k3s"] = "ok" if k3s_ok else "GAP"

        # --- Vercel ------------------------------------------------------
        function, variable = SERVICES[owner]["vercel"]
        vercel_ok = True
        if function not in functions:
            problems.append(
                f"vercel: {method} {path} is owned by {owner}, and there is no {function} Function "
                f"for it"
            )
            vercel_ok = False
        if variable and variable not in variables:
            problems.append(
                f"vercel: {method} {path} is owned by {owner}, and {variable} is not among the "
                f"deployment's documented environment variables"
            )
            vercel_ok = False
        if not any(rule.startswith("/v1/") or rule.startswith("/admin/v1/") for rule in rewrites):
            problems.append(
                f"vercel: {method} {path} is owned by {owner}, and no rewrite routes the public "
                f"paths to the gateway"
            )
            vercel_ok = False
        row["vercel"] = "ok" if vercel_ok else "GAP"

        rows.append(row)

    # The matrix is the deliverable: it says, path by path, whether both
    # topologies serve it.
    width = max(len(r["path"]) for r in rows) + 2
    print("contract path".ljust(width) + "owner".ljust(12) + "k3s".ljust(8) + "vercel")
    for row in rows:
        print(
            f"{row['method'] + ' ' + row['path']}".ljust(width)
            + str(row["owner"]).ljust(12)
            + str(row["k3s"]).ljust(8)
            + str(row["vercel"])
        )

    if problems:
        print()
        print(f"topology coverage check: {len(problems)} problem(s)")
        for problem in problems:
            print(f"  - {problem}")
        return 1

    print()
    print(
        f"topology coverage check: OK ({len(check_set)} paths, "
        f"{len(deployed)} k3s services, {len(functions)} vercel functions)"
    )
    return 0


if __name__ == "__main__":
    raise SystemExit(main())
