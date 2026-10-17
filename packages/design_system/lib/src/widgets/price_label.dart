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
  });

  final Money money;
  final TextStyle? style;
  final bool showCurrency;

  @override
  Widget build(BuildContext context) {
    final base = style ?? Theme.of(context).textTheme.titleMedium;
    final formatted = showCurrency
        ? '${money.format()} ${money.currency.code}'
        : money.format();
    // Screen readers get the spoken form; a symbol alone is useless.
    return Semantics(
      label: '${money.amountMinor / 100} ${money.currency.code}',
      excludeSemantics: true,
      child: Text(
        formatted,
        style: (base ?? const TextStyle()).copyWith(
          fontWeight: FontWeight.w700,
          color: GemColors.ink,
        ),
      ),
    );
  }
}
