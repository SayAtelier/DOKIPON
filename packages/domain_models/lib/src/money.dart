import 'package:meta/meta.dart';

/// ISO-4217 currency codes the storefront supports.
///
/// `exponent` is the number of minor-unit digits. JPY has none.
enum Currency {
  jpy('JPY', 0, '¥', 0),
  usd('USD', 2, r'$', 2),
  eur('EUR', 2, '€', 2),
  cny('CNY', 2, 'CN¥', 2),
  twd('TWD', 2, r'NT$', 2);

  const Currency(this.code, this.exponent, this.symbol, this.decimalDigits);

  final String code;
  final int exponent;
  final String symbol;
  final int decimalDigits;

  /// Defensive parse: an unknown code from the server never crashes a client,
  /// it falls back to a defensive default the UI can still render.
  static Currency parse(String? raw) {
    if (raw == null) return Currency.jpy;
    final upper = raw.toUpperCase();
    for (final c in Currency.values) {
      if (c.code == upper) return c;
    }
    return Currency.jpy;
  }
}

/// An amount in the currency's smallest unit.
///
/// Mirrors `commerce_core::Money`: construction rejects negatives, arithmetic is
/// overflow-checked, and cross-currency addition is an error rather than a
/// silent coercion.
@immutable
class Money {
  /// Const constructor with validation done by assertion. Call
  /// [Money.tryOf] or [Money.of] when the amount is not a literal.
  const Money(this.amountMinor, this.currency) : assert(amountMinor >= 0);

  /// Safe constructor. Returns `null` for a negative amount rather than
  /// clamping it — a negative price is a contract violation, not a discount.
  static Money? tryOf(int amountMinor, Currency currency) {
    if (amountMinor < 0) return null;
    return Money(amountMinor, currency);
  }

  factory Money.of(int amountMinor, Currency currency) {
    final m = tryOf(amountMinor, currency);
    if (m == null) {
      throw ArgumentError.value(
        amountMinor,
        'amountMinor',
        'money must not be negative',
      );
    }
    return m;
  }

  final int amountMinor;
  final Currency currency;

  bool get isZero => amountMinor == 0;

  /// Zero amount in a given currency.
  const Money.zero(Currency currency) : this(0, currency);

  Money operator +(Money other) {
    if (other.currency != currency) {
      throw ArgumentError('cannot add ${other.currency.code} to ${currency.code}');
    }
    return Money(amountMinor + other.amountMinor, currency);
  }

  /// Rounded at the minor-unit boundary, matching the server's
  /// `Money::checked_mul_rounded`.
  Money multipliedBy(int quantity) {
    if (quantity < 0) {
      throw ArgumentError.value(quantity, 'quantity', 'must not be negative');
    }
    return Money(amountMinor * quantity, currency);
  }

  /// Display string. Never used for business math — only for rendering.
  String format() {
    final sym = currency.symbol;
    if (currency.decimalDigits == 0) {
      return '$sym$amountMinor';
    }
    final whole = amountMinor ~/ 100;
    final frac = (amountMinor % 100).abs().toString().padLeft(2, '0');
    return '$sym$whole.$frac';
  }

  Map<String, dynamic> toJson() => {
        'amountMinor': amountMinor,
        'currency': currency.code,
      };

  static Money fromJson(Map<String, dynamic> json) {
    final currency = Currency.parse(json['currency'] as String?);
    final raw = json['amountMinor'];
    final amount = raw is int ? raw : int.tryParse('$raw') ?? 0;
    return Money.of(amount, currency);
  }

  @override
  bool operator ==(Object other) =>
      other is Money && other.amountMinor == amountMinor && other.currency == currency;

  @override
  int get hashCode => Object.hash(amountMinor, currency);

  @override
  String toString() => 'Money($format, raw=$amountMinor, $rawCurrency)';

  String get rawCurrency => currency.code;
}
