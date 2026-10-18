/*
@cypher schema=1 source_sha256=0fcbbb233006abd1b5aa8efa4a6460766316d896a9af2da7a1b8385e47b10cfc
MERGE (self:File {path:"packages/api_client/lib/api_client.dart"})
MERGE (api:Symbol {id:"packages/api_client/lib/api_client.dart::api_client", kind:"module"})
MERGE (client:Symbol {id:"packages/api_client/lib/src/admin_api_client.dart::AdminApiClient", kind:"class"})
MERGE (models:Symbol {id:"packages/api_client/lib/src/admin_models.dart::admin_models", kind:"module"})
MERGE (self)-[:DEFINES]->(api)
MERGE (self)-[:EXPOSES]->(client)
MERGE (self)-[:EXPOSES]->(models)
@endcypher
*/
/// HTTP client for the GemArc Commerce BFF.
///
/// Three rules from the design atlas are enforced here rather than left to
/// callers:
///
/// 1. **Writes carry an `Idempotency-Key`.** The client generates one UUID per
///    logical operation and reuses it across transport-level retries, so a
///    retried POST can never become two charges.
/// 2. **Problems are `ApiProblem`, not exceptions with a stack trace.** Callers
///    branch on `code` / `retryable`.
/// 3. **`payment_unknown` is never auto-retried by re-submission.** The checkout
///    poll loop re-reads status instead; re-submitting could double-charge.
library;

export 'src/commerce_api_client.dart';
export 'src/admin_api_client.dart';
export 'src/admin_models.dart';
export 'src/idempotency_key.dart';
export 'src/liff_bridge.dart';
