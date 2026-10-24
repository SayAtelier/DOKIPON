#!/usr/bin/env python3
"""check-manifest-secrets.py — the k3s manifests must not lie about credentials.

Two classes of defect, both of which happened for real:

1. **A secret key that is not there.** The admin Deployment read
   `OPERATOR_USER` / `OPERATOR_PASSWORD` / `VIEWER_USER` / `VIEWER_PASSWORD`
   from a Secret that did not define them. A `secretKeyRef` to a key that
   does not exist stops the container from starting at all — so the admin
   console never came up, and nothing in the manifest said so.

2. **A development credential in a committed file.** The development
   password, the development signing key and the development webhook secret
   are published in this repository. A manifest that carries one of them
   hands the deployment to anyone who has read the source. The services
   refuse to start while a development value is in place, so a manifest that
   carries one does not even deploy — but the check is here so the mistake
   is caught before it reaches a cluster.

Run:

    python tools/commerce-e2e/scripts/check-manifest-secrets.py
    python tools/commerce-e2e/scripts/check-manifest-secrets.py <manifest dir>

Needs PyYAML. Exits 1 with the list of problems when there are any.
"""

from __future__ import annotations

import sys
from pathlib import Path

try:
    import yaml
except ImportError:  # pragma: no cover - a missing dependency, not a defect
    print("PyYAML is required: pip install pyyaml", file=sys.stderr)
    raise SystemExit(2)

ROOT = Path(__file__).resolve().parents[3]
MANIFESTS = (
    Path(sys.argv[1]).resolve()
    if len(sys.argv) > 1
    else ROOT / "services" / "deploy" / "k3s"
)

# The development values, as the services define them. A committed manifest
# that carries one of these is shipping a published credential.
DEV_VALUES = {
    "dev-only-admin": "ADMIN_PASSWORD",
    "dev-only-key": "LINE_ID_TOKEN_KEY",
    "dev-only-secret": "PSP_WEBHOOK_SECRET",
    "dev-channel": "LINE_CHANNEL_ID",
}

# The credentials each service refuses to start without.
REQUIRED_BY_SERVICE = {
    "commerce-identity": ["LINE_CHANNEL_ID", "LINE_ID_TOKEN_KEY"],
    "commerce-payment-adapter": ["PSP_WEBHOOK_SECRET"],
    "commerce-admin": ["ADMIN_PASSWORD"],
}


def load_documents(path: Path) -> list[dict]:
    documents = []
    for document in yaml.safe_load_all(path.read_text(encoding="utf-8")):
        if isinstance(document, dict):
            documents.append(document)
    return documents


def secret_keys(secret: dict) -> set[str]:
    """The keys a Secret defines, from `stringData` or `data`."""
    keys: set[str] = set()
    for field in ("stringData", "data"):
        value = secret.get(field)
        if isinstance(value, dict):
            keys |= set(value)
    return keys


def walk(node, path: str = ""):
    """Yield every (path, value) pair in a document."""
    if isinstance(node, dict):
        for key, value in node.items():
            yield from walk(value, f"{path}.{key}" if path else str(key))
    elif isinstance(node, list):
        for index, value in enumerate(node):
            yield from walk(value, f"{path}[{index}]")
    else:
        yield path, node


def main() -> int:
    problems: list[str] = []

    secrets: dict[str, set[str]] = {}
    deployments: list[tuple[str, dict]] = []
    all_documents: list[tuple[str, dict]] = []

    for manifest in sorted(MANIFESTS.glob("*.yaml")):
        for document in load_documents(manifest):
            all_documents.append((manifest.name, document))
            kind = document.get("kind")
            name = (document.get("metadata") or {}).get("name")
            if kind == "Secret" and name:
                secrets[name] = secret_keys(document)
            if kind == "Deployment" and name:
                deployments.append((name, document))

    # 1. Every secretKeyRef points at a Secret that exists, and at a key that
    #    is in it — unless the reference is marked optional.
    for deployment_name, deployment in deployments:
        pod = deployment.get("spec", {}).get("template", {}).get("spec", {})
        for container in pod.get("containers", []):
            for entry in container.get("env", []):
                source = entry.get("valueFrom") or {}
                reference = source.get("secretKeyRef")
                if not reference:
                    continue
                secret_name = reference.get("name")
                key = reference.get("key")
                if secret_name not in secrets:
                    problems.append(
                        f"{deployment_name}: env {entry.get('name')} reads "
                        f"{secret_name}/{key}, and no Secret named {secret_name} is defined"
                    )
                    continue
                if key not in secrets[secret_name] and not reference.get("optional"):
                    problems.append(
                        f"{deployment_name}: env {entry.get('name')} reads "
                        f"{secret_name}/{key}, and that Secret does not define the key - "
                        f"the container cannot start. Mark the reference optional, or define the key."
                    )

    # 2. Every service that needs a credential is given it.
    for deployment_name, deployment in deployments:
        needed = REQUIRED_BY_SERVICE.get(deployment_name)
        if not needed:
            continue
        pod = deployment.get("spec", {}).get("template", {}).get("spec", {})
        provided: set[str] = set()
        for container in pod.get("containers", []):
            for entry in container.get("env", []):
                if entry.get("name"):
                    provided.add(entry["name"])
            for source in container.get("envFrom", []) or []:
                reference = (source.get("secretRef") or {}).get("name")
                if reference and reference in secrets:
                    provided |= secrets[reference]
        for variable in needed:
            if variable not in provided:
                problems.append(
                    f"{deployment_name}: {variable} is not provided, and the service "
                    f"refuses to start without it"
                )

    # 3. No committed manifest carries a development credential.
    for manifest_name, document in all_documents:
        for path, value in walk(document):
            if not isinstance(value, str):
                continue
            if value in DEV_VALUES:
                problems.append(
                    f"{manifest_name}: {path} holds the development value `{value}` "
                    f"({DEV_VALUES[value]}). A value in a committed manifest is a published "
                    f"value - the service refuses to start with it, and a reader of this "
                    f"repository can use it against any deployment that has not replaced it."
                )

    if problems:
        print(f"manifest secret check: {len(problems)} problem(s)")
        for problem in problems:
            print(f"  - {problem}")
        return 1

    print(
        f"manifest secret check: OK "
        f"({len(deployments)} deployments, {len(secrets)} secrets)"
    )
    return 0


if __name__ == "__main__":
    raise SystemExit(main())
