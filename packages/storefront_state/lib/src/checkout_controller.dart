import 'dart:async';

import 'package:api_client/api_client.dart';
import 'package:domain_models/domain_models.dart';

import 'catalog_controller.dart';

/// Drives a checkout and owns the one rule that protects the customer from a
/// double charge:
///
/// > While a checkout is unresolved (`payment_unknown`, `confirming`), the
/// > client **polls the status endpoint**. It must never re-submit the
/// > checkout, because the server cannot tell whether the first attempt
/// > captured.
///
/// [submit] is called exactly once per logical attempt. Retries inside the HTTP
/// layer reuse the same `Idempotency-Key`, so even a transport-level retry is a
/// replay on the server.
class CheckoutController with ChangeNotifierLike {
  CheckoutController(this._api);

  final CommerceApiClient _api;

  CheckoutSnapshot? _snapshot;
  ApiProblem? _error;
  bool _submitting = false;
  int _pollAttempt = 0;
  Timer? _pollTimer;
  StorefrontConfig _config = StorefrontConfig.fallback(StoreChannel.web);

  CheckoutSnapshot? get snapshot => _snapshot;
  ApiProblem? get error => _error;
  bool get isSubmitting => _submitting;
  CheckoutStatus get status => _snapshot?.status ?? CheckoutStatus.new_;

  bool get isPending => status.isPending;
  bool get isConfirmed => status.isSuccess;
  bool get isRejected => status == CheckoutStatus.rejected;

  /// True when the UI should say "we're checking with your bank" rather than
  /// "something went wrong".
  bool get isAwaitingPaymentProvider => status == CheckoutStatus.paymentUnknown;

  /// Set the server-provided poll budget. Called once at boot.
  void configure(StorefrontConfig config) {
    _config = config;
  }

  /// `operation` identifies the logical checkout (e.g. `checkout.cart-1`) so a
  /// retry replays instead of re-charging.
  Future<void> submit({
    required String operation,
    required Map<String, dynamic> payload,
  }) async {
    if (_submitting) return;
    _submitting = true;
    _error = null;
    _pollAttempt = 0;
    notify();
    try {
      final result = await _api.submitCheckout(
        operation: operation,
        payload: payload,
      );
      _snapshot = result;
      _schedulePollIfNeeded();
    } on ApiProblem catch (p) {
      // An idempotency conflict means "you changed the payload"; a fresh
      // operation is required, and that is the user's explicit new action.
      if (p.isIdempotencyConflict) {
        _error = p;
        _submitting = false;
        notify();
        return;
      }
      _error = p;
    } finally {
      _submitting = false;
      notify();
    }
  }

  void _schedulePollIfNeeded() {
    _pollTimer?.cancel();
    if (!isPending) return;
    if (_pollAttempt >= _config.maxPollAttempts) {
      // Budget exhausted. Tell the user to check orders rather than spinning
      // forever — never claim failure, never claim success.
      _error = const ApiProblem(
        type: 'client.poll_budget_exhausted',
        title: 'Still processing',
        status: 0,
        code: 'client.poll_budget_exhausted',
        detail:
            'The payment provider has not answered yet. Check your orders shortly; '
            'you have not been charged twice.',
        traceId: '',
        retryable: false,
      );
      notify();
      return;
    }
    _pollTimer = Timer(Duration(milliseconds: _config.statusPollIntervalMs), poll);
  }

  /// Re-read authoritative status. Safe to call at any time.
  Future<void> poll() async {
    final current = _snapshot;
    if (current == null || current.status.isTerminal) return;
    _pollAttempt++;
    try {
      _snapshot = await _api.checkoutStatus(current.checkoutId);
      _error = null;
    } on ApiProblem catch (p) {
      // A transient poll failure is not a failed checkout. Keep polling within
      // the budget; surface the problem without changing the status.
      _error = p;
    }
    notify();
    _schedulePollIfNeeded();
  }

  /// Retry after a transport-level error. This re-reads status; it never
  /// re-submits the checkout.
  Future<void> recover() async {
    final current = _snapshot;
    if (current == null) return;
    await poll();
  }

  @override
  void dispose() {
    _pollTimer?.cancel();
    super.dispose();
  }
}
