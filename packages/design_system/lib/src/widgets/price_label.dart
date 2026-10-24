import 'package:domain_models/domain_models.dart';
import 'package:flutter/material.dart';

import '../tokens.dart';

/// Renders a [Money] using its own currency formatting.
///
/// Every amount on screen goes through here, which guarantees no widget
/// accidentally prints a raw `double` of minor units.
class PriceLabel extends StatelessWidget {
  const PriceLabel({
    super.key,
    required this.money,
    this.style,
    this.showCurrency = false,
    this.color,
    this.showStrikethrough = false,
    this.strikethroughColor,
  });

  final Money money;
  final TextStyle? style;
  final bool showCurrency;
  final Color? color;
  final bool showStrikethrough;
  final Color? strikethroughColor;

  @override
  Widget build(BuildContext context) {
    final base = style ?? Theme.of(context).textTheme.titleSmall;
    final formatted = showCurrency
        ? '${money.format()} ${money.currency.code}'
        : money.format();
    // Screen readers get the spoken form: the amount in the currency's own
    // major unit, and the code. A symbol alone is useless — and so is a
    // number that is not the price: the amount is in minor units, and the
    // currencies that have no minor units (JPY) are already the price.
    return Semantics(
      label: '${money.spokenAmount} ${money.currency.code}',
      excludeSemantics: true,
      child: Text(
        formatted,
        style: (base ?? const TextStyle()).copyWith(
          fontWeight: FontWeight.w600,
          color: color ?? DokColors.ink,
          decoration: showStrikethrough ? TextDecoration.lineThrough : null,
          decorationColor: strikethroughColor ?? DokColors.inkFaint,
          decorationThickness: 1.5,
        ),
      ),
    );
  }
}

/// A price with the original struck through next to it.
///
/// This is the reference storefront's single most recognisable price
/// treatment: the old price in small muted type with a line through it, the
/// new price larger and in the accent colour. It is also the only place a
/// discount can be shown, so the struck price and the current price must
/// travel together — a lone sale price is meaningless.
class PriceRow extends StatelessWidget {
  const PriceRow({
    super.key,
    required this.money,
    this.originalMoney,
  });

  final Money money;

  /// The pre-discount amount. Null when the product is not discounted.
  final Money? originalMoney;

  @override
  Widget build(BuildContext context) {
    final theme = Theme.of(context);
    final discounted = originalMoney != null &&
        originalMoney!.amountMinor > money.amountMinor &&
        originalMoney!.currency == money.currency;

    if (!discounted) {
      return PriceLabel(
        money: money,
        style: theme.textTheme.titleLarge, // 14px serif
      );
    }

    return Wrap(
      crossAxisAlignment: WrapCrossAlignment.end,
      spacing: DokSpace.sm,
      children: [
        PriceLabel(
          money: originalMoney!,
          style: theme.textTheme.bodySmall,
          color: DokColors.inkFaint,
          showStrikethrough: true,
        ),
        PriceLabel(
          money: money,
          style: theme.textTheme.titleMedium?.copyWith(fontSize: 15),
          color: DokColors.crimson, // was accent
        ),
      ],
    );
  }
}
