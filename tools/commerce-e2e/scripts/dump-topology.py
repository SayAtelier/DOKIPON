"""dump-topology.py — print what each deployment topology actually declares.

For the k3s side: every Deployment and Service, its port, and the env it
gets. For the Vercel side: the Functions, the rewrites, and the env
variables the project is configured with.

The output is the raw material for the coverage matrix: a contract path is
only really implemented in a topology if the service that owns it is
deployed there, on the port the routing table points at, with the
environment it needs.
"""

from __future__ import annotations

import json
import re
import sys
from pathlib import Path

import yaml

ROOT = Path(__file__).resolve().parents[3]
K3S = ROOT / "services" / "deploy" / "k3s"


def k3s() -> dict:
    deployments: dict[str, dict] = {}
    services: dict[str, dict] = {}
    for manifest in sorted(K3S.glob("*.yaml")):
        for document in yaml.safe_load_all(manifest.read_text(encoding="utf-8")):
            if not isinstance(document, dict):
                continue
            kind = document.get("kind")
            name = (document.get("metadata") or {}).get("name")
            if kind == "Deployment" and name:
                containers = (
                    document.get("spec", {})
                    .get("template", {})
                    .get("spec", {})
                    .get("containers", [])
                )
                env = []
                env_from = []
                for container in containers:
                    for entry in container.get("env", []) or []:
                        source = entry.get("valueFrom") or {}
                        if "secretKeyRef" in source:
                            env.append(
                                f"{entry['name']}<secret:{source['secretKeyRef']['name']}"
                                f"/{source['secretKeyRef'].get('key', '?')}>"
                            )
                        elif "configMapKeyRef" in source:
                            env.append(
                                f"{entry['name']}<configmap:{source['configMapKeyRef']['name']}"
                                f"/{source['configMapKeyRef'].get('key', '?')}>"
                            )
                        else:
                            env.append(f"{entry['name']}={entry.get('value', '')}")
                    for source in container.get("envFrom", []) or []:
                        if "configMapRef" in source:
                            env_from.append(f"configmap:{source['configMapRef']['name']}")
                        if "secretRef" in source:
                            env_from.append(f"secret:{source['secretRef']['name']}")
                deployments[name] = {
                    "ports": [p.get("containerPort") for p in containers[0].get("ports", [])]
                    if containers
                    else [],
                    "env": env,
                    "env_from": env_from,
                    "file": manifest.name,
                }
            elif kind == "Service" and name:
                ports = [
                    {"port": p.get("port"), "target": p.get("targetPort")}
                    for p in document.get("spec", {}).get("ports", [])
                ]
                services[name] = {"ports": ports, "file": manifest.name}
    return {"deployments": deployments, "services": services}


def configmap_env() -> dict:
    """The variables the shared ConfigMap hands to every service."""
    out: dict[str, str] = {}
    for manifest in sorted(K3S.glob("*.yaml")):
        for document in yaml.safe_load_all(manifest.read_text(encoding="utf-8")):
            if isinstance(document, dict) and document.get("kind") == "ConfigMap":
                for key, value in (document.get("data") or {}).items():
                    out[key] = value
    return out


def vercel() -> dict:
    root = Path(ROOT)
    vercel_json = json.loads((root / "vercel.json").read_text(encoding="utf-8"))
    functions = sorted(p.name for p in (root / "api").rglob("*.rs"))
    env_example = (root / "services" / "deploy" / "vercel" / "env.example").read_text(
        encoding="utf-8"
    )
    variables = []
    for line in env_example.splitlines():
        match = re.match(r"^([A-Z][A-Z0-9_]*)=", line)
        if match:
            variables.append(match.group(1))
    return {
        "rewrites": vercel_json.get("rewrites", []),
        "functions": functions,
        "env_variables": variables,
    }


if __name__ == "__main__":
    what = sys.argv[1] if len(sys.argv) > 1 else "all"
    payload: dict = {}
    if what in ("all", "k3s"):
        payload["k3s"] = k3s()
        payload["k3s_configmap"] = configmap_env()
    if what in ("all", "vercel"):
        payload["vercel"] = vercel()
    print(json.dumps(payload, indent=2, ensure_ascii=False))
