import 'package:design_system/design_system.dart';
import 'package:domain_models/domain_models.dart';
import 'package:flutter/material.dart';

/// Stock state, always paired with a text label.
///
/// Colour alone is not an accessible signal, so each variant renders an
/// explicit ja/en word. The reference storefront keeps stock state as plain
/// muted text rather than a coloured pill — a dozen coloured pills in a grid
/// would fight the product photography, so the badge is quiet text and only
/// the genuinely urgent case (almost gone) is tinted.
class AvailabilityBadge extends StatelessWidget {
  const AvailabilityBadge({super.key, required this.availability, this.dense = false});

  final Availability availability;
  final bool dense;

  static String labelFor(Availability a) => switch (a) {
        Availability.inStock => '在庫あり / In stock',
        Availability.lowStock => '残りわずか / Low stock',
        Availability.outOfStock => '在庫切れ / Out of stock',
        Availability.madeToOrder => '受注生産 / Made to order',
      };

  static Color colorFor(Availability a) => switch (a) {
        Availability.inStock => DokColors.inkFaint,
        Availability.lowStock => DokColors.warning,
        Availability.outOfStock => DokColors.inkFaint,
        Availability.madeToOrder => DokColors.inkFaint,
      };

  /// Out of stock is not purchasable.
  bool get isPurchasable => availability != Availability.outOfStock;

  @override
  Widget build(BuildContext context) {
    final color = colorFor(availability);
    return Semantics(
      label: labelFor(availability),
      excludeSemantics: true,
      child: Text(
        labelFor(availability),
        style: Theme.of(context).textTheme.labelMedium?.copyWith(
              color: color,
              fontWeight: FontWeight.w500,
            ),
      ),
    );
  }
}
