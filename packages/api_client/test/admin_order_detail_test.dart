import 'dart:convert';

import 'package:api_client/api_client.dart';
import 'package:domain_models/domain_models.dart';
import 'package:flutter_test/flutter_test.dart';

/// The order detail the order service actually returns.
///
/// Captured from the deployed service, not written from the contract: the
/// point of this test is to prove the client can read **what the server
/// sends**, and a document written from the contract would only prove the
/// client can read what the contract says.
const String orderDetailJson = '''
{
  "awaitingReconciliation": false,
  "lines": [
    {"currency": "JPY", "quantity": 1, "sku": "sku-bra-01",
     "unitPrice": {"amountMinor": 8900, "currency": "JPY"}}
  ],
  "orderId": "ord-000001",
  "reservationId": "res-000003",
  "state": "confirmed",
  "timeline": [
    {"event": "Requested", "from": "new", "sequence": 1, "to": "new"},
    {"event": "BeginReserve", "from": "new", "sequence": 2, "to": "reserving"},
    {"event": "StockReserved", "from": "reserving", "sequence": 3, "to": "stock_reserved"},
    {"event": "RequestPayment", "from": "stock_reserved", "sequence": 4, "to": "awaiting_payment"},
    {"event": "PaymentAuthorized", "from": "awaiting_payment", "sequence": 5, "to": "confirming"},
    {"event": "Confirmed", "from": "confirming", "sequence": 6, "to": "confirmed"}
  ],
  "total": {"amountMinor": 8900, "currency": "JPY"}
}
''';

/// The operator console's order detail screen used to crash on this
/// document.
///
/// Two field names were wrong, and both were wrong in the quiet way: the
/// line's price was a flat `unit_amount_minor` that the client does not look
/// for, so the line's amount came out as zero — and then, because the
/// client insists on a currency, it threw outright. And the timeline's
/// transitions were looked up as `fromState` / `from_state` while the
/// service has always sent `from` / `to`, so every step of the trail read
/// "unknown → unknown".
///
/// The document above is the service's own answer, so a client that cannot
/// read it fails here rather than in the browser.
void main() {
  group('AdminOrderDetail', () {
    late AdminOrderDetail detail;

    setUp(() {
      detail = AdminOrderDetail.fromJson(
        jsonDecode(orderDetailJson) as Map<String, dynamic>,
      );
    });

    test('reads the order itself', () {
      expect(detail.orderId, 'ord-000001');
      expect(detail.state, CheckoutStatus.confirmed);
      expect(detail.total.amountMinor, 8900);
      expect(detail.total.currency, Currency.jpy);
    });

    test('reads the line\'s price as a Money', () {
      expect(detail.lines, hasLength(1));
      final line = detail.lines.first;
      expect(line.sku, 'sku-bra-01');
      expect(line.quantity, 1);
      expect(line.unitPrice.amountMinor, 8900);
      expect(line.unitPrice.currency, Currency.jpy);
      expect(line.lineTotal.amountMinor, 8900);
    });

    test('reads the timeline as the transitions that happened', () {
      expect(detail.trail, hasLength(6));
      expect(detail.trail.first.from, CheckoutStatus.new_);
      expect(detail.trail.first.to, CheckoutStatus.new_);
      expect(detail.trail[2].from, CheckoutStatus.reserving);
      expect(detail.trail[2].to, CheckoutStatus.stockReserved);
      expect(detail.trail.last.from, CheckoutStatus.confirming);
      expect(detail.trail.last.to, CheckoutStatus.confirmed);
    });
  });
}
