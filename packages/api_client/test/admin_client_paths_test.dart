import 'dart:convert';

import 'package:api_client/api_client.dart';
import 'package:flutter_test/flutter_test.dart';
import 'package:http/http.dart' as http;
import 'package:http/testing.dart';

/// The admin client's **paths**.
///
/// The console's order detail screen used to ask for
/// `/admin/v1/admin/orders/{id}` and get a 404 with an empty body, so the
/// screen said "Error" and offered a Retry that could not work — the
/// operator could not open an order, and could not reconcile one.
///
/// Two faults sat on top of each other: the path had the old draft's shape
/// (`v1/admin/orders/…` — the shape the service does not serve), and it was
/// built with `Uri.replace(pathSegments: …)`, which produces a path with no
/// leading slash, so the browser resolved it against the console's own URL
/// (`/admin/`) and doubled the prefix.
///
/// The paths are pinned here so that neither can come back.
void main() {
  group('AdminApiClient paths', () {
    late Uri requested;

    AdminApiClient client() {
      final httpClient = MockClient((request) async {
        requested = request.url;
        // A body the client can parse: the test is about the **path**, and a
        // body the client rejects would fail the test for the wrong reason.
        final body = request.url.path.endsWith('/orders')
            ? jsonEncode({'items': []})
            : jsonEncode({
                'orderId': 'ord-000001',
                'state': 'confirmed',
                'awaitingReconciliation': false,
                'total': {'amountMinor': 8900, 'currency': 'JPY'},
                'lines': [
                  {
                    'sku': 'sku-bra-01',
                    'quantity': 1,
                    'unitPrice': {'amountMinor': 8900, 'currency': 'JPY'},
                    'lineTotal': {'amountMinor': 8900, 'currency': 'JPY'},
                  }
                ],
                'timeline': [],
              });
        return http.Response(
          body,
          200,
          headers: {'content-type': 'application/json'},
        );
      });
      return AdminApiClient(
        baseUri: Uri.parse('/'),
        httpClient: httpClient,
      );
    }

    test('the order detail is asked for under the path the facade serves',
        () async {
      await client().orderDetail('ord-000001');
      expect(requested.path, '/admin/v1/orders/ord-000001');
    });

    test('the reconcile is asked for under the path the facade serves',
        () async {
      await client().reconcileOrder(orderId: 'ord-000001', authorized: true);
      expect(requested.path, '/admin/v1/orders/ord-000001/reconcile');
    });

    test('the order list is asked for under the same shape', () async {
      await client().listOrders();
      expect(requested.path, '/admin/v1/orders');
    });

    test('the path is absolute, so the browser cannot resolve it against '
        'the console\'s own URL', () async {
      await client().orderDetail('ord-000001');
      expect(requested.path.startsWith('/'), isTrue,
          reason: 'a path without a leading slash is resolved against the page');
    });
  });
}
