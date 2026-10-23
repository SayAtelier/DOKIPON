# CNPG migration notes (proposed)

Document ID: `RGS-PLAT-CNPG-001`, v0.1 proposed. Derived from ARC-008,
ADR-0052, and DEC-009; the proposed document has not been reviewed.

- The legacy endpoint `postgres:5432` is replaced by CNPG's read-write Service
  `rgs-postgres-rw:5432`.
- The Cluster name is `rgs-postgres`; CNPG manages a superuser Secret named
  `rgs-postgres-superuser`. The old `postgres-superuser` Secret uses different
  key names and is not wired into `superuserSecret`.
- The six role password Secrets remain `player-db-credentials`,
  `economy-db-credentials`, `match-db-credentials`, `social-db-credentials`,
  `admin-db-credentials`, and `cluster-ops-db-credentials`. `managed.roles`
  references them. Their `url` values must change to the new service hostname;
  the existing `DATABASE_URL` `secretKeyRef` names can remain as they are.
- Service manifests such as `01-player-service.yaml` through
  `06-cluster-ops-service.yaml` read `DATABASE_URL` from those Secrets. Keep
  each reference aligned with its existing per-domain credential Secret and
  update only the stored endpoint as part of cutover.
- CNPG provisions a new 5Gi PVC. It does not reuse `postgres-data-pvc`; preserve
  the old PVC until dump/restore or reseed verification passes.
- CNPG-managed physical backups use an object-store destination. A raw PVC
  destination is not available through the built-in `destinationPath` field.
  `backup.retentionPolicy` is deprecated in CNPG 1.30 and does not enable a
  backup target by itself. No `volumeSnapshot` is configured. Automated backup
  remains an open item until a PVC-backed object-store endpoint or a separately
  managed backup job is specified.
- The six `0001_*.sql` files are currently NO-GO placeholders without DDL. They
  must not be run as schema setup until approvals and reviewed migrations exist.

Recommended old-object cleanup after a verified cutover: delete the old
`Deployment/postgres`, then `Service/postgres`, old `NetworkPolicy`, old
`ConfigMap`, and finally `postgres-data-pvc`. Keep all six per-domain role
Secrets while the CNPG Cluster references them.
