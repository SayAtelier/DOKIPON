import 'package:api_client/api_client.dart';
import 'package:design_system/design_system.dart';
import 'package:domain_models/domain_models.dart';
import 'package:flutter/material.dart';
import 'package:flutter_test/flutter_test.dart';
import 'package:storefront_state/storefront_state.dart';

/// "Show the whole collection" has to say "there is nothing to clear" when
/// nothing is filtered.
///
/// The button was always enabled. In the initial state — no category applied
/// — there is nothing to clear, so pressing it did nothing: a control that
/// is painted, announced as a button, and inert. The audit found it on the
/// live deployment.
///
/// A disabled button is the honest version: it says "not right now", and the
/// audit's rule for a deliberately disabled control is to skip it rather
/// than report it as broken.
void main() {
  testWidgets('the button is disabled until a category narrows the catalogue',
      (tester) async {
    final api = _FakeCatalogue();
    final catalog = CatalogController(api);

    await tester.pumpWidget(_screen(api, catalog));
    await tester.pumpAndSettle();

    // Nothing is filtered, so there is nothing to clear.
    expect(_shopAllButton(tester).onPressed, isNull);

    // A category narrows the catalogue, and now the button has work to do.
    await tester.tap(find.text('SWIM'));
    await tester.pump();
    await tester.pump();
    expect(_shopAllButton(tester).onPressed, isNotNull);

    // And pressing it puts the whole collection back.
    await tester.tap(find.widgetWithText(OutlinedButton, 'SHOP COLLECTION'));
    await tester.pump();
    await tester.pump();
    expect(catalog.categoryId, isNull);
  });
}

OutlinedButton _shopAllButton(WidgetTester tester) => tester.widget<OutlinedButton>(
      find.widgetWithText(OutlinedButton, 'SHOP COLLECTION'),
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
class _FakeCatalogue extends CommerceApiClient {
  _FakeCatalogue()
      : super(
          baseUri: Uri.parse('https://example.test'),
          channel: StoreChannel.web,
        );

  @override
  Future<ProductPage> listProducts({
    String? cursor,
    int limit = 20,
    String locale = 'ja',
    String? categoryId,
  }) async {
    return const ProductPage(items: []);
  }
}
