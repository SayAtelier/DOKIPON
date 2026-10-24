import 'package:design_system/design_system.dart';
import 'package:domain_models/domain_models.dart';
import 'package:flutter/material.dart';
import 'package:flutter/semantics.dart';
import 'package:flutter_test/flutter_test.dart';

/// The product card is a button, and it says so.
///
/// The card used to be a bare `InkWell` with no `Semantics` around it,
/// while every other tappable widget in this package — the cart, the
/// utility icons, the category strip, the cart drawer's item — declares
/// `Semantics(button: true, ...)`. A bare `InkWell` gives no stable role:
/// Flutter exposed the card as a button before the grid settled and as a
/// group afterwards, so its role depended on the scroll position.
///
/// A shopper who cannot see the screen pays for that: a control whose
/// role is a group is not announced as activatable.
void main() {
  group('ProductCard', () {
    ProductSummary summary({
      String title = 'DOKIPON 霞 キャミソール',
      Money price = const Money(6900, Currency.jpy),
      Availability availability = Availability.madeToOrder,
    }) =>
        ProductSummary(
          productId: 'p-adult-01',
          sku: 'sku-cami-01',
          title: title,
          price: price,
          availability: availability,
        );

    Future<void> pump(WidgetTester tester, ProductSummary product,
        {VoidCallback? onAdd}) async {
      // The card is a grid tile: the photograph is 1:1 with the tile's
      // width, so a card left at the test viewport's full width overflows.
      // 284 is the width the storefront's grid gives it.
      await tester.pumpWidget(
        MaterialApp(
          home: Scaffold(
            body: Center(
              child: SizedBox(
                width: 284,
                child: ProductCard(
                  product: product,
                  onOpen: () {},
                  onAdd: onAdd,
                ),
              ),
            ),
          ),
        ),
      );
    }

    testWidgets('is announced as a button', (tester) async {
      final handle = tester.ensureSemantics();
      await pump(tester, summary());

      final semantics = tester.getSemantics(find.byType(ProductCard));
      expect(semantics.hasFlag(SemanticsFlag.isButton), isTrue,
          reason: 'the card is an InkWell with an onTap, so it has to say so');

      handle.dispose();
    });

    testWidgets('says the name, the price and the stock state', (tester) async {
      final handle = tester.ensureSemantics();
      await pump(tester, summary());

      final label = tester.getSemantics(find.byType(ProductCard)).label;
      expect(label, contains('DOKIPON 霞 キャミソール'));
      // The price in the currency's own major unit: a JPY price is already
      // the price. Announcing a fraction of it is the bug this package
      // already fixed once, in `PriceLabel`.
      expect(label, contains('6900 JPY'));
      expect(label, isNot(contains('69 JPY')));
      expect(label, contains('受注生産 / Made to order'));

      handle.dispose();
    });

    testWidgets('the add-to-cart button keeps its own semantics',
        (tester) async {
      final handle = tester.ensureSemantics();
      await pump(tester, summary(), onAdd: () {});

      // The card excludes the text inside it so that it is announced once,
      // but the add-to-cart button is a separate control and has to stay
      // reachable. A card that swallowed it would leave the shopper with
      // no way to buy.
      final button = find.widgetWithText(FilledButton, 'カートに追加');
      expect(button, findsOneWidget);
      expect(
        tester.getSemantics(button).hasFlag(SemanticsFlag.isButton),
        isTrue,
      );

      handle.dispose();
    });
  });
}
