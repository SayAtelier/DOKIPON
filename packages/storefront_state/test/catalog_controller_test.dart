import 'dart:convert';

import 'package:api_client/api_client.dart';
import 'package:domain_models/domain_models.dart';
import 'package:flutter_test/flutter_test.dart';
import 'package:http/http.dart' as http;
import 'package:http/testing.dart';
import 'package:storefront_state/storefront_state.dart';

/// A fake catalogue behind the real [CommerceApiClient].
///
/// It answers `/v1/products` the way the server does — it filters on the
/// `categoryId` the request carries and pages with the cursor — and it
/// records every query string, so a test can assert both what the client
/// asked for and what it did with the answer.
class FakeCatalog {
  FakeCatalog({this.pageSize = 20});

  /// How many products one page carries.
  final int pageSize;

  /// The query string of every request the client made.
  final List<Uri> requests = [];

  /// The products the fake holds, per category. A category that is not in
  /// the map holds nothing — that is the "empty category" case.
  final Map<String, List<String>> catalogue = {
    'cat-lingerie': ['p-brief-01', 'p-cami-01', 'p-lace-01'],
    'cat-swim': ['p-swim-01'],
  };

  /// Set to answer with a server error instead of a page.
  int? failWithStatus;

  /// The ids the unfiltered catalogue holds.
  List<String> get everything =>
      catalogue.values.expand((ids) => ids).toList(growable: false);

  http.Client client() => MockClient((request) async {
        requests.add(request.url);

        if (failWithStatus != null) {
          return http.Response(
            jsonEncode({
              'type': 'about:blank',
              'title': 'Boom',
              'status': failWithStatus,
              'code': 'catalog.failed',
              'detail': 'the fake catalogue is down',
              'traceId': 'trace-1',
              'retryable': false,
            }),
            failWithStatus!,
            headers: {'content-type': 'application/json'},
          );
        }

        final category = request.url.queryParameters['categoryId'];
        final ids =
            category == null ? everything : (catalogue[category] ?? const <String>[]);
        final offset =
            int.tryParse(request.url.queryParameters['cursor'] ?? '0') ?? 0;
        final page = ids.skip(offset).take(pageSize).toList(growable: false);
        final next = offset + page.length;

        return http.Response(
          jsonEncode({
            'items': [for (final id in page) _summary(id)],
            'next_cursor': next < ids.length ? '$next' : null,
          }),
          200,
          headers: {'content-type': 'application/json'},
        );
      });

  static Map<String, dynamic> _summary(String id) => {
        'productId': id,
        'sku': 'sku-$id',
        'title': 'Product $id',
        'price': {'amountMinor': 4800, 'currency': 'JPY'},
        'availability': 'in_stock',
      };
}

CatalogController controllerFor(FakeCatalog fake) => CatalogController(
      CommerceApiClient(
        baseUri: Uri.parse('https://example.test'),
        channel: StoreChannel.web,
        httpClient: fake.client(),
      ),
    );

void main() {
  group('CatalogController', () {
    test('an unfiltered load asks for the whole catalogue', () async {
      final fake = FakeCatalog();
      final controller = controllerFor(fake);

      await controller.load();

      expect(fake.requests, hasLength(1));
      expect(fake.requests.single.queryParameters['categoryId'], isNull);
      expect(controller.categoryId, isNull);
      expect(controller.items.map((p) => p.productId), fake.everything);
    });

    test('selecting a category re-queries with the filter', () async {
      final fake = FakeCatalog();
      final controller = controllerFor(fake);

      await controller.load();
      await controller.selectCategory('cat-swim');

      expect(fake.requests, hasLength(2));
      expect(fake.requests.last.queryParameters['categoryId'], 'cat-swim');
      expect(controller.categoryId, 'cat-swim');
    });

    test('selecting a category replaces the list instead of appending to it',
        () async {
      final fake = FakeCatalog();
      final controller = controllerFor(fake);

      await controller.load();
      expect(controller.items.map((p) => p.productId), fake.everything);

      await controller.selectCategory('cat-swim');

      // The products of the previous query are not part of this one.
      expect(controller.items.map((p) => p.productId), ['p-swim-01']);
    });

    test('the cursor is reset by a category change', () async {
      final fake = FakeCatalog(pageSize: 2);
      final controller = controllerFor(fake);

      await controller.load();
      expect(controller.hasMore, isTrue);

      await controller.selectCategory('cat-lingerie');

      // The first page of the new category starts from the beginning: no
      // cursor from the old query.
      expect(fake.requests.last.queryParameters.containsKey('cursor'), isFalse);
      expect(controller.items.map((p) => p.productId),
          ['p-brief-01', 'p-cami-01']);
    });

    test('loadMore carries the category filter and appends to it', () async {
      final fake = FakeCatalog(pageSize: 2);
      final controller = controllerFor(fake);

      await controller.selectCategory('cat-lingerie');
      expect(controller.items.map((p) => p.productId),
          ['p-brief-01', 'p-cami-01']);

      await controller.loadMore();

      expect(fake.requests.last.queryParameters['categoryId'], 'cat-lingerie');
      expect(fake.requests.last.queryParameters['cursor'], '2');
      expect(controller.items.map((p) => p.productId),
          ['p-brief-01', 'p-cami-01', 'p-lace-01']);
    });

    test('a category with no products is an empty list, not an error',
        () async {
      final fake = FakeCatalog();
      final controller = controllerFor(fake);

      await controller.selectCategory('cat-sale');

      expect(controller.items, isEmpty);
      expect(controller.error, isNull);
      expect(controller.isEmpty, isTrue);
    });

    test('a failed page never wipes the products already on screen',
        () async {
      final fake = FakeCatalog(pageSize: 2);
      final controller = controllerFor(fake);

      await controller.selectCategory('cat-lingerie');
      expect(controller.items, hasLength(2));

      fake.failWithStatus = 500;
      await controller.loadMore();

      // The pages on screen survive the failure — only the error is new.
      expect(controller.items.map((p) => p.productId),
          ['p-brief-01', 'p-cami-01']);
      expect(controller.error, isNotNull);
      expect(controller.error!.code, 'catalog.failed');
    });

    test('selecting a category again is a no-op', () async {
      final fake = FakeCatalog();
      final controller = controllerFor(fake);

      await controller.selectCategory('cat-swim');
      await controller.selectCategory('cat-swim');

      expect(fake.requests, hasLength(1));
      expect(controller.items.map((p) => p.productId), ['p-swim-01']);
    });

    test('a category can be cleared and the whole catalogue comes back',
        () async {
      final fake = FakeCatalog();
      final controller = controllerFor(fake);

      await controller.selectCategory('cat-swim');
      await controller.selectCategory(null);

      expect(fake.requests.last.queryParameters.containsKey('categoryId'),
          isFalse);
      expect(controller.categoryId, isNull);
      expect(controller.items.map((p) => p.productId), fake.everything);
    });
  });
}
