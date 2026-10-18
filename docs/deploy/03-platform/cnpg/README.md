# CloudNativePG for Rust Game Server (proposed)

Document ID: `RGS-PLAT-CNPG-001`, v0.1 proposed. Derived from ARC-008,
ADR-0052, and DEC-009; the proposed document has not been reviewed.

## Scope and prerequisites

This bundle defines the `rust-game-server` namespace, a single-instance CNPG
Cluster named `rgs-postgres`, and ingress policy for its Pods. The Cluster
creates `player_db`, `economy_db`, `match_db`, `social_db`, `admin_db`, and
`cluster_ops_db`, with their matching `*_user` roles. It uses the legacy image,
storage class, storage size, and resource values.

The CloudNativePG operator and CRD must already be installed. The six existing
`*-db-credentials` Secrets from
`docs/deploy/01-k8s-manifests/20-postgres-secret.yaml` must exist in
`rust-game-server` before the Cluster is created. Each must provide the existing
`username` and `password` keys; do not print or copy credential values into a
manifest. The old `postgres-superuser` Secret is not reused because its key
names do not match CNPG's superuser Secret format. With superuser access
enabled, CNPG manages `rgs-postgres-superuser`.

CNPG creates the `rgs-postgres-rw` read-write service (and `-ro`/`-r` read-only
services). This replaces the old `postgres` Service endpoint. The network policy
selects CNPG Pods by `cnpg.io/cluster: rgs-postgres` and preserves the old three
client component selectors on TCP 5432.

No `monitoring` stanza is included: the legacy Deployment has no Prometheus
scrape annotation, metrics port, or exporter configuration.

## Relationship to manifests 20–25

This directory is the proposed replacement deployment entry point for legacy
files 20–25. The existing credential objects from file 20 remain prerequisites
and must be retained while CNPG roles use them. Do not include the old
StatefulSet/Deployment, Service, ConfigMap, PVC, or NetworkPolicy in the new
apply set. The new Cluster provisions its own PVC; it does not attach
`postgres-data-pvc`.

## Migrating an existing database

CNPG initializes a fresh data volume. Applying this bundle does not copy data
from the old `postgres-data-pvc` and must not be treated as a data migration.

For a data-preserving cutover:

1. Capture an independent backup of each of the six old databases before
   stopping writes. Use a custom-format `pg_dump` per database so each dump can
   be restored into the already-created CNPG database. Keep the old PVC until
   the new data has been checked and the backup is retained elsewhere.
2. Stop or scale down all database-writing applications, then stop the old
   PostgreSQL workload. Keep the old PVC, old credentials, and backup files.
3. Apply this Kustomize bundle and wait for `rgs-postgres` to become Ready.
   Restore each dump into its corresponding CNPG database, using the matching
   `*_user` as the restore role. Do not restore cluster-wide database/role
   definitions over the CNPG bootstrap objects.
4. Update each existing connection Secret's `url` value to use
   `rgs-postgres-rw:5432`, preserving its database and credential. Restart the
   consuming Deployments and check their database readiness.
5. Compare pre-cutover and post-restore row counts or application-level
   invariants for the tables that contain data. Only after that check and an
   independent backup succeeds, remove the old objects in this order:
   `Deployment/postgres`, `Service/postgres`, `NetworkPolicy/postgres-ingress`,
   `ConfigMap/postgres-config`, then `PVC/postgres-data-pvc`. Retain the six
   role Secrets. The old PVC is the last item to remove because it is the
   simplest rollback source.

For a development reseed instead, apply the Cluster, recreate data using the
approved application seed process, and verify expected records and application
health before deleting the old PVC. The current `0001_*.sql` files are explicit
NO-GO placeholders with no DDL; they do not seed a schema or business data.

## Applying the database migration files

The files under `docs/deploy/03-db-migrations/*/0001_*.sql` are currently
NO-GO placeholders. Do not execute them until their stated approval conditions
are met and they contain reviewed DDL. Once approved, apply each file to its
matching database as the CNPG superuser, for example from PowerShell at the
repository root:

```powershell
$pod = 'rgs-postgres-1'
$migration = 'docs/deploy/03-db-migrations/player_db/0001_initial_placeholder.sql'
Get-Content -Raw -Encoding UTF8 $migration |
  kubectl exec -i -n rust-game-server $pod -- psql -U postgres -d player_db -v ON_ERROR_STOP=1
```

Repeat for `economy_db`, `match_db`, `social_db`, `admin_db`, and
`cluster_ops_db`, matching each directory to its database name. The database
endpoint for applications is `rgs-postgres-rw:5432`. Existing per-domain
connection Secret names stay the same in this design; update their `url` values
and keep the `DATABASE_URL` `secretKeyRef` names in the service manifests
aligned with those Secrets.

## Backup limitation

CNPG 1.30's built-in `backup.barmanObjectStore.destinationPath` is an
object-store URL, not a filesystem path on a PVC. A PVC destination requires a
separately operated S3-compatible/object-store service backed by a PVC, plus
its endpoint and credentials, or a separate backup job. This task has no
existing object-store endpoint or credentials to reference. The Cluster keeps
`backup.retentionPolicy: "30d"` from the request, but CNPG 1.30 deprecates this
field and, without a configured backup target, it does not schedule backups.
The backup requirement is therefore unresolved and must be completed before
relying on automated backups. `volumeSnapshot` is not configured for this
single-instance cluster.

## Deploy and verify

Run from the repository root after the required Secrets exist:

```powershell
kubectl apply -k docs/deploy/03-platform/cnpg
kubectl wait --for=condition=Ready cluster/rgs-postgres -n rust-game-server --timeout=10m
kubectl get cluster,pods,svc,pvc -n rust-game-server
kubectl exec -n rust-game-server rgs-postgres-1 -- psql -U postgres -d postgres -Atc "SELECT datname FROM pg_database WHERE datname IN ('player_db','economy_db','match_db','social_db','admin_db','cluster_ops_db') ORDER BY datname;"
kubectl exec -n rust-game-server rgs-postgres-1 -- psql -U postgres -d postgres -Atc "SELECT rolname FROM pg_roles WHERE rolname IN ('player_user','economy_user','match_user','social_user','admin_user','cluster_ops_user') ORDER BY rolname;"
kubectl get networkpolicy rgs-postgres-ingress -n rust-game-server
```

These are documentation commands only; no cluster command was run while
authoring this bundle.
