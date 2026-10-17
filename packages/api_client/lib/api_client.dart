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
export 'src/idempotency_key.dart';
export 'src/liff_bridge.dart';
