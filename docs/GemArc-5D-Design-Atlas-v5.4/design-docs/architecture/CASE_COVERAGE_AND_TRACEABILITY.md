# フルスタック正常／異常・組合せ Case 設計

> Model-level 144 Case は Guard/Transition の独立検証用。実決済、サービス障害、Flutter 実機を検証した証拠ではない。追加の E2E Case 群は `reference-models/engineering_cases.json` を正とする。

## 期待する Case レベル

| Test level | target | must assert |
|---|---|---|
| UT | Dart DTO/guards, service domain, scene LHC transform | deterministic, boundary, typed errors |
| Contract | OpenAPI and events schema | backwards compatibility, schema evolution |
| IT | Catalog/Order/Inventory/Payments with mock | transactional invariants, idempotence |
| ST | BFF + services + Flutter Web | E2E states, error hints, fraud policy |
| Mobile UAT | Android/iOS | lifecycle, offline, deep link, accessibility |
| LINE UAT | LIFF WebView + external browser | login cancel, capability fallback, policy |
| Chaos | broker DB payment fault and recovery | no duplicate charge, outbox/reconcile, DLQ |
| 3D–5D | same scene/graph across layers | stable IDs, LHC, trace diff and replay |

## ケースの判断原則

正常、異常、境界、復旧、複合（複数原因）、セキュリティ、ユーザー行動、データ同期を分離。各 Case に `preconditions → injection → expectedState → invariants → testLevel → graphRefs → requirementId → evidence` を記入。CausalRun が success でも server transaction と client UI が不整合なら合格としない。
