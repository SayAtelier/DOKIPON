import 'package:api_client/api_client.dart';
import 'package:design_system/design_system.dart';
import 'package:domain_models/domain_models.dart';
import 'package:flutter/material.dart';
import 'package:flutter_test/flutter_test.dart';
import 'package:storefront_state/storefront_state.dart';

/// The category strip is a filter, and it has to reach the catalogue.
///
/// The strip used to be a row of `onTap: () {}` — painted, focusable,
/// announced to a screen reader as buttons, and completely inert. These
/// tests pin the two halves of the fix: the tap reaches the controller, and
/// the controller's filter is what the catalogue is asked for.
///
/// The catalogue answers empty on purpose. The product card overflows its
/// grid tile by ~33px (the grid allows 96px for the text block, the card
/// needs more), and that layout defect is not what this test is about —
/// putting a product on screen here would drown the assertion in an
/// unrelated overflow error.
void main() {
  testWidgets('tapping a category filters the catalogue', (tester) async {
    final api = _FakeCatalogue();
    final catalog = CatalogController(api);

    await tester.pumpWidget(_screen(api, catalog));
    await tester.pumpAndSettle();

    // The whole strip is on screen.
    expect(find.text('LINGERIE'), findsOneWidget);
    expect(find.text('SWIM'), findsOneWidget);

    await tester.tap(find.text('SWIM'));
    await tester.pump();
    await tester.pump();

    // The tap reached the controller...
    expect(catalog.categoryId, 'cat-swim');
    // ...and the controller asked the server for that category.
    expect(api.categoryIds, ['cat-swim']);
    // The screen says which category it is showing.
    expect(find.text('SWIM'), findsNWidgets(2)); // the strip and the heading
  });

  testWidgets('the applied category is marked in the strip', (tester) async {
    final api = _FakeCatalogue();
    final catalog = CatalogController(api);

    await tester.pumpWidget(_screen(api, catalog));
    await tester.pumpAndSettle();

    expect(tester.widget<Text>(navEntry('LINGERIE')).style?.color,
        DokColors.ink);

    await tester.tap(find.text('SWIM'));
    await tester.pump();
    await tester.pump();

    // The applied entry is marked; the others stay quiet.
    expect(tester.widget<Text>(navEntry('SWIM')).style?.color, DokColors.crimson);
    expect(tester.widget<Text>(navEntry('LINGERIE')).style?.color, DokColors.ink);
  });

  testWidgets('a category with no products shows a message, not a blank page',
      (tester) async {
    final api = _FakeCatalogue();
    final catalog = CatalogController(api);

    await tester.pumpWidget(_screen(api, catalog));
    await tester.pumpAndSettle();

    await tester.tap(find.text('BEAUTY'));
    await tester.pump();
    await tester.pump();

    expect(catalog.categoryId, 'cat-beauty');
    // The shopper is told what is empty — the screen is not blank.
    expect(find.textContaining('No products in BEAUTY'), findsOneWidget);
  });
}

/// The strip's entry for [label].
///
/// Once a category is applied its label also appears in the section rule, so
/// the finder has to be scoped to the strip — the entry is the label inside
/// the strip's `InkWell`.
Finder navEntry(String label) => find.descendant(
      of: find
          .ancestor(of: find.text(label), matching: find.byType(InkWell))
          .first,
      matching: find.text(label),
    );

/// The storefront screen with a catalogue that answers from memory.
Widget _screen(CommerceApiClient api, CatalogController catalog) =>
    MaterialApp(
      theme: DokTheme.light(),
      home: CatalogScreen(
        session: SessionController(api: api),
        catalog: catalog,
        cart: CartController(api),
        checkout: CheckoutController(api),
      ),
    );

/// A catalogue that answers from memory, so no test ever touches the network.
///
/// It records the `categoryId` of every request and answers with an empty
/// page — the case the storefront has to render a message for.
class _FakeCatalogue extends CommerceApiClient {
  _FakeCatalogue()
      : super(
          baseUri: Uri.parse('https://example.test'),
          channel: StoreChannel.web,
        );

  /// The `categoryId` of every request the client made.
  final List<String?> categoryIds = [];

  @override
  Future<ProductPage> listProducts({
    String? cursor,
    int limit = 20,
    String locale = 'ja',
    String? categoryId,
  }) async {
    categoryIds.add(categoryId);
    return const ProductPage(items: []);
  }
}
