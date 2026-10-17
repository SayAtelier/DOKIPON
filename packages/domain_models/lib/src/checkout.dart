import 'package:meta/meta.dart';

import 'money.dart';

/// Server-side checkout state, mirroring `commerce_core::CheckoutState`.
///
/// The naming here is deliberately identical to the server so a status string
/// can be mapped 1:1 with an exhaustive `switch`, and an unknown future state
/// degrades to [CheckoutStatus.unknown] instead of throwing.
enum CheckoutStatus {
  new_('new'),
  reserving('reserving'),
  stockReserved('stock_reserved'),
  awaitingPayment('awaiting_payment'),
  paymentUnknown('payment_unknown'),
  confirming('confirming'),
  confirmed('confirmed'),
  rejected('rejected'),
  cancelled('cancelled'),
  unknown('unknown');

  const CheckoutStatus(this.wire);

  final String wire;

  static CheckoutStatus parse(String? raw) {
    if (raw == null) return CheckoutStatus.unknown;
    for (final s in CheckoutStatus.values) {
      if (s.wire == raw) return s;
    }
    return CheckoutStatus.unknown;
  }

  /// A state the client should stop polling.
  bool get isTerminal => switch (this) {
        CheckoutStatus.confirmed ||
        CheckoutStatus.rejected ||
        CheckoutStatus.cancelled =>
          true,
        _ => false,
      };

  /// `payment_unknown` is explicitly NOT terminal. The server is reconciling
  /// with the payment provider; re-submitting would risk a double charge.
  bool get isPollable => switch (this) {
        CheckoutStatus.awaitingPayment ||
        CheckoutStatus.paymentUnknown ||
        CheckoutStatus.confirming =>
          true,
        _ => false,
      };

  /// Whether the UI must keep the customer waiting rather than showing an error.
  bool get isPending => !isTerminal && isPollable;

  bool get isSuccess => this == CheckoutStatus.confirmed;
}

/// One line of an order.
@immutable
class OrderLine {
  const OrderLine({
    required this.sku,
    required this.quantity,
    required this.unitPrice,
  });

  final String sku;
  final int quantity;

  /// The **server-authoritative** unit price captured at checkout time. The
  /// client's own subtotal is never used for display of the charged amount.
  final Money unitPrice;

  Money get lineTotal => unitPrice.multipliedBy(quantity);

  factory OrderLine.fromJson(Map<String, dynamic> json) => OrderLine(
        sku: json['sku'] as String? ?? '',
        quantity: (json['quantity'] as num?)?.toInt() ?? 0,
        unitPrice: Money.fromJson(
          Map<String, dynamic>.from(json['unitAmount'] as Map? ?? const {}),
        ),
      );
}

/// A point on the order's timeline (the design atlas's 4D trace).
@immutable
class TrailEntry {
  const TrailEntry({
    required this.sequence,
    required this.event,
    required this.from,
    required this.to,
  });

  final int sequence;
  final String event;
  final CheckoutStatus from;
  final CheckoutStatus to;

  factory TrailEntry.fromJson(Map<String, dynamic> json) => TrailEntry(
        sequence: (json['sequence'] as num?)?.toInt() ?? 0,
        event: json['event'] as String? ?? '',
        from: CheckoutStatus.parse(json['from_state'] as String? ?? json['fromState'] as String?),
        to: CheckoutStatus.parse(json['to_state'] as String? ?? json['toState'] as String?),
      );
}

/// The authoritative checkout status from `GET /v1/checkouts/{id}`.
@immutable
class CheckoutSnapshot {
  const CheckoutSnapshot({
    required this.checkoutId,
    required this.status,
    this.orderId,
    this.awaitingReconciliation = false,
  });

  final String checkoutId;
  final CheckoutStatus status;
  final String? orderId;
  final bool awaitingReconciliation;

  factory CheckoutSnapshot.fromJson(Map<String, dynamic> json) =>
      CheckoutSnapshot(
        checkoutId: json['checkoutId'] as String? ?? '',
        status: CheckoutStatus.parse(json['status'] as String?),
        orderId: json['orderId'] as String?,
        awaitingReconciliation:
            json['awaitingReconciliation'] as bool? ?? false,
      );

  CheckoutSnapshot copyWith({CheckoutStatus? status, bool? awaitingReconciliation}) =>
      CheckoutSnapshot(
        checkoutId: checkoutId,
        status: status ?? this.status,
        orderId: orderId,
        awaitingReconciliation:
            awaitingReconciliation ?? this.awaitingReconciliation,
      );
}
