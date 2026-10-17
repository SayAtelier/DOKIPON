import 'package:design_system/design_system.dart';
import 'package:domain_models/domain_models.dart';
import 'package:flutter/material.dart';

/// Stock state, always paired with a text label.
///
/// Colour alone is not an accessible signal, so each variant renders an
/// explicit ja/en word. This is what the atlas's UI spec means by
/// "cognitive load" — the badge answers "can I buy this?" without a legend.
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
        Availability.inStock => GemColors.success,
        Availability.lowStock => GemColors.warning,
        Availability.outOfStock => GemColors.danger,
        Availability.madeToOrder => GemColors.info,
      };

  /// Out of stock is not purchasable.
  bool get isPurchasable => availability != Availability.outOfStock;

  @override
  Widget build(BuildContext context) {
    final color = colorFor(availability);
    return Semantics(
      label: labelFor(availability),
      excludeSemantics: true,
      child: Container(
        padding: EdgeInsets.symmetric(
          horizontal: dense ? GemSpace.sm : GemSpace.md,
          vertical: dense ? 2 : 4,
        ),
        decoration: BoxDecoration(
          color: color.withValues(alpha: 0.10),
          borderRadius: const BorderRadius.all(GemRadius.sm),
          border: Border.all(color: color.withValues(alpha: 0.35)),
        ),
        child: Text(
          labelFor(availability),
          style: Theme.of(context).textTheme.labelMedium?.copyWith(
                color: color,
                fontWeight: FontWeight.w600,
              ),
        ),
      ),
    );
  }
}
