import 'package:design_system/design_system.dart';
import 'package:domain_models/domain_models.dart';
import 'package:flutter/material.dart';
import 'package:flutter_test/flutter_test.dart';

/// A price a screen reader says out loud has to be the price.
///
/// The label used to divide the amount by 100 whatever the currency was.
/// That is right for a currency with minor units and wrong for one without:
/// JPY has none, so an ¥8900 item was announced as "89 JPY" — a hundredth of
/// the price, to the shopper who cannot see the ¥8900 on screen.
///
/// The visible text was always right, which is why nobody looking at the
/// screen ever saw it.
void main() {
  group('PriceLabel', () {
    Future<String?> spokenLabel(WidgetTester tester, Money money) async {
      final handle = tester.ensureSemantics();
      await tester.pumpWidget(
        MaterialApp(home: Scaffold(body: PriceLabel(money: money))),
      );
      final label = tester.getSemantics(find.byType(PriceLabel)).label;
      handle.dispose();
      return label;
    }

    testWidgets('says the price, not a fraction of it', (tester) async {
      expect(await spokenLabel(tester, Money(8900, Currency.jpy)), '8900 JPY');
      expect(await spokenLabel(tester, Money(1800, Currency.jpy)), '1800 JPY');
      expect(await spokenLabel(tester, Money(4900, Currency.jpy)), '4900 JPY');
    });

    testWidgets('a currency with minor units is said in its major unit',
        (tester) async {
      expect(await spokenLabel(tester, Money(8900, Currency.usd)), '89.00 USD');
    });

    testWidgets('the visible text is the price with its symbol', (tester) async {
      await tester.pumpWidget(
        MaterialApp(
          home: Scaffold(body: PriceLabel(money: Money(8900, Currency.jpy))),
        ),
      );
      expect(find.text('¥8900'), findsOneWidget);
    });
  });
}
