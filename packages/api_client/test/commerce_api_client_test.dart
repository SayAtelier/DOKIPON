import 'dart:convert';

import 'package:api_client/api_client.dart';
import 'package:domain_models/domain_models.dart';
import 'package:flutter_test/flutter_test.dart';
import 'package:http/http.dart' as http;
import 'package:http/testing.dart';

/// The catalogue route's query string is the contract's business.
///
/// `contracts/openapi.yaml` declares the category filter on
/// `GET /v1/products` as `categoryId`. The server reads that name literally,
/// so a client that spells the parameter any other way gets a catalogue
/// that is not filtered — with no error to show for it.
void main() {
  group('listProducts', () {
    test('sends the category filter under the name the contract declares',
        () async {
      late Uri requested;
      final client = _client((request) {
        requested = request.url;
        return _page([
          {
            'productId': 'p-swim-01',
            'sku': 'sku-swim-01',
            'title': 'スイムビーフ / Swim brief',
            'price': {'amountMinor': 5900, 'currency': 'JPY'},
            'availability': 'in_stock',
          },
        ]);
      });

      final page = await client.listProducts(categoryId: 'cat-swim');

      expect(requested.queryParameters['categoryId'], 'cat-swim');
      expect(page.items.single.productId, 'p-swim-01');
    });

    test('omits the filter when no category is asked for', () async {
      late Uri requested;
      final client = _client((request) {
        requested = request.url;
        return _page(const []);
      });

      await client.listProducts();

      // An empty parameter is not the same as no parameter: the server would
      // filter for a category called "".
      expect(requested.queryParameters.containsKey('categoryId'), isFalse);
    });

    test('keeps the filter and the cursor on the same request', () async {
      late Uri requested;
      final client = _client((request) {
        requested = request.url;
        return _page(const []);
      });

      await client.listProducts(cursor: '2', categoryId: 'cat-lingerie');

      expect(requested.queryParameters['cursor'], '2');
      expect(requested.queryParameters['categoryId'], 'cat-lingerie');
    });
  });
}

/// A client whose transport is [answer].
CommerceApiClient _client(http.Response Function(http.Request request) answer) =>
    CommerceApiClient(
      baseUri: Uri.parse('https://example.test'),
      channel: StoreChannel.web,
      httpClient: MockClient((request) async => answer(request)),
    );

/// One page of products, in the shape the contract specifies.
http.Response _page(List<Map<String, dynamic>> items) => http.Response(
      jsonEncode({'items': items, 'next_cursor': null}),
      200,
      headers: {'content-type': 'application/json'},
    );
