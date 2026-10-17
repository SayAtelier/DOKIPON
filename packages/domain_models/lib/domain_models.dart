/// Shared domain models for the GemArc Commerce clients (App / Web / LINE MINI App).
///
/// These mirror the server contracts in `services/crates/*` and
/// `contracts/openapi.yaml`. Two rules from the design atlas are load-bearing
/// and encoded here rather than left to the UI:
///
/// 1. **Money is integer minor units + ISO-4217 currency.** No `double` ever
///    holds a business amount. [Money] is the only type a price can be.
/// 2. **`payment_unknown` is not failure and not success.** [CheckoutStatus]
///    models it as its own case, and the poll loop must keep polling.
library;

export 'src/checkout.dart';
export 'src/money.dart';
export 'src/product.dart';
export 'src/problem.dart';
export 'src/session.dart';
