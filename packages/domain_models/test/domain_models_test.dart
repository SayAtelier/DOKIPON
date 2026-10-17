import 'package:domain_models/domain_models.dart';
import 'package:flutter_test/flutter_test.dart';

/// The client mirrors the server's invariants so a UI bug cannot produce a
/// request the server would (correctly) reject, or display an amount the server
/// would not honour.
void main() {
  group('Money', () {
    test('holds integer minor units, never a double', () {
      final m = Money.of(4800, Currency.jpy);
      expect(m.amountMinor, 4800);
      expect(m.amountMinor, isA<int>());
    });

    test('rejects a negative amount instead of clamping', () {
      expect(Money.tryOf(-1, Currency.usd), isNull);
      expect(() => Money.of(-1, Currency.usd), throwsArgumentError);
    });

    test('JPY formats with no decimals and USD with two', () {
      expect(Money.of(4800, Currency.jpy).format(), '¥4800');
      expect(Money.of(4800, Currency.usd).format(), r'$48.00');
    });

    test('multiplies quantity', () {
      expect(Money.of(4800, Currency.jpy).multipliedBy(3).amountMinor, 14400);
    });

    test('rejects negative quantity', () {
      expect(
        () => Money.of(4800, Currency.jpy).multipliedBy(-1),
        throwsArgumentError,
      );
    });

    test('refuses cross-currency addition', () {
      expect(
        () => Money.of(1, Currency.usd) + Money.of(1, Currency.jpy),
        throwsArgumentError,
      );
    });

    test('unknown currency code falls back defensively', () {
      expect(Currency.parse('XYZ'), Currency.jpy);
      expect(Currency.parse(null), Currency.jpy);
      expect(Currency.parse('usd'), Currency.usd);
    });

    test('round-trips through JSON', () {
      final m = Money.of(12800, Currency.jpy);
      expect(Money.fromJson(m.toJson()), m);
    });

    test('zero is well defined', () {
      expect(Money.zero(Currency.jpy).amountMinor, 0);
      expect(Money.zero(Currency.jpy).isZero, isTrue);
    });
  });

  group('CheckoutStatus', () {
    test('payment_unknown is neither terminal nor a failure', () {
      const s = CheckoutStatus.paymentUnknown;
      expect(s.isTerminal, isFalse);
      expect(s.isPollable, isTrue);
      expect(s.isPending, isTrue);
      expect(s.isSuccess, isFalse);
    });

    test('confirmed and rejected are terminal', () {
      expect(CheckoutStatus.confirmed.isTerminal, isTrue);
      expect(CheckoutStatus.rejected.isTerminal, isTrue);
      expect(CheckoutStatus.cancelled.isTerminal, isTrue);
    });

    test('an unknown server state degrades instead of throwing', () {
      expect(CheckoutStatus.parse('quantum_superposition'), CheckoutStatus.unknown);
      expect(CheckoutStatus.parse(null), CheckoutStatus.unknown);
      expect(CheckoutStatus.parse('payment_unknown'), CheckoutStatus.paymentUnknown);
    });
  });

  group('ApiProblem', () {
    test('an idempotency conflict must not be blindly retried', () {
      const p = ApiProblem(
        type: 'about:blank',
        title: 'Conflict',
        status: 409,
        code: 'checkout.idempotency_conflict',
        detail: '',
        traceId: '',
        retryable: false,
      );
      expect(p.isIdempotencyConflict, isTrue);
      expect(p.retryable, isFalse);
    });

    test('an unresolved payment must not trigger a resubmit', () {
      const p = ApiProblem(
        type: 'about:blank',
        title: 'Service Unavailable',
        status: 503,
        code: 'payment.unknown',
        detail: '',
        traceId: 't',
        retryable: true,
      );
      expect(p.isUnresolvedPayment, isTrue);
    });

    test('a cart version conflict is refetch-and-retry, not overwrite', () {
      const p = ApiProblem(
        type: 'about:blank',
        title: 'Conflict',
        status: 409,
        code: 'cart.version_conflict',
        detail: '',
        traceId: '',
        retryable: false,
      );
      expect(p.isVersionConflict, isTrue);
    });

    test('parses a server problem body', () {
      final p = ApiProblem.fromJson(const {
        'status': 403,
        'code': 'order.forbidden',
        'detail': 'order belongs to another session',
        'traceId': 'abc',
        'retryable': false,
      });
      expect(p.status, 403);
      expect(p.isForbidden, isTrue);
      expect(p.traceId, 'abc');
    });
  });

  group('Session', () {
    test('a guest can browse but not check out', () {
      final g = Session.guest('guest-1');
      expect(g.isGuest, isTrue);
      expect(g.canCheckout, isFalse);
      expect(g.isValidAt(DateTime.now().millisecondsSinceEpoch), isFalse);
    });

    test('a line user session is valid inside its window', () {
      final now = 1_000_000;
      final s = Session(
        sessionId: 'sess-1',
        kind: SessionKind.lineUser,
        expiresAtMs: now + 60_000,
      );
      expect(s.canCheckout, isTrue);
      expect(s.isValidAt(now), isTrue);
      expect(s.isValidAt(now + 60_001), isFalse);
    });
  });

  group('Product', () {
    test('a policy-restricted product is not eligible for LINE or App', () {
      final d = ProductDetail.fromJson(const {
        'productId': 'p-adult-01',
        'sku': 'sku-adult-01',
        'price': {'amountMinor': 9800, 'currency': 'JPY'},
        'version': 1,
        'stockPolicy': 'reserved',
        'availability': 'in_stock',
        'channelEligibility': {'web': true, 'app': false, 'line_mini_app': false},
      });
      expect(d.eligibility.allows('line'), isFalse);
      expect(d.eligibility.allows('app'), isFalse);
      expect(d.eligibility.allows('web'), isTrue);
    });

    test('localization falls back ja then any', () {
      final d = ProductDetail.fromJson(const {
        'productId': 'p',
        'sku': 's',
        'price': {'amountMinor': 100, 'currency': 'JPY'},
        'version': 1,
        'stockPolicy': 'reserved',
        'availability': 'in_stock',
        'titleI18n': {'ja': 'JP', 'en': 'EN'},
      });
      expect(d.localizedTitle('ja'), 'JP');
      expect(d.localizedTitle('fr'), 'JP');
      expect(d.localizedTitle('en'), 'EN');
    });

    test('only reserved stock policy needs the inventory step', () {
      expect(StockPolicy.reserved.needsInventory, isTrue);
      expect(StockPolicy.unbounded.needsInventory, isFalse);
      expect(StockPolicy.madeToOrder.needsInventory, isFalse);
    });

    test('a page reports whether more results exist', () {
      final page = ProductPage.fromJson(const {
        'items': [
          {
            'productId': 'p1',
            'sku': 's1',
            'title': 'T',
            'price': {'amountMinor': 4800, 'currency': 'JPY'},
            'availability': 'in_stock',
          },
        ],
        'next_cursor': '1',
      });
      expect(page.items, hasLength(1));
      expect(page.hasMore, isTrue);
    });
  });

  group('Cart', () {
    test('snapshot subtotal is display-only and sums correctly', () {
      const cart = Cart(
        cartId: 'cart-1',
        version: 3,
        lines: [
          CartLine(sku: 'a', quantity: 2, unitAmountSnapshot: 4800),
          CartLine(sku: 'b', quantity: 1, unitAmountSnapshot: 12800),
        ],
      );
      expect(cart.snapshotSubtotal.amountMinor, 22400);
      expect(cart.version, 3);
    });

    test('an empty cart is empty', () {
      const cart = Cart(cartId: 'c', version: 1, lines: []);
      expect(cart.isEmpty, isTrue);
    });
  });
}
