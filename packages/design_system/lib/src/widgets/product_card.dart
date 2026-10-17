import 'package:design_system/design_system.dart';
import 'package:domain_models/domain_models.dart';
import 'package:flutter/material.dart';

/// A catalogue row. Shared verbatim by App, Web and LINE so the three channels
/// cannot drift visually.
class ProductCard extends StatelessWidget {
  const ProductCard({
    super.key,
    required this.product,
    required this.onOpen,
    this.onAdd,
  });

  final ProductSummary product;
  final VoidCallback onOpen;
  final VoidCallback? onAdd;

  @override
  Widget build(BuildContext context) {
    final purchasable = product.availability != Availability.outOfStock;
    return Card(
      margin: const EdgeInsets.symmetric(
        horizontal: GemSpace.md,
        vertical: GemSpace.xs,
      ),
      child: InkWell(
        onTap: onOpen,
        borderRadius: const BorderRadius.all(GemRadius.lg),
        child: Padding(
          padding: const EdgeInsets.all(GemSpace.md),
          child: Row(
            crossAxisAlignment: CrossAxisAlignment.start,
            children: [
              Expanded(
                child: Column(
                  crossAxisAlignment: CrossAxisAlignment.start,
                  children: [
                    Text(
                      product.title,
                      style: Theme.of(context).textTheme.titleMedium,
                    ),
                    const SizedBox(height: GemSpace.xs),
                    PriceLabel(money: product.price),
                    const SizedBox(height: GemSpace.sm),
                    AvailabilityBadge(availability: product.availability, dense: true),
                  ],
                ),
              ),
              const SizedBox(width: GemSpace.md),
              if (onAdd != null)
                FilledButton(
                  // Out-of-stock is not purchasable, so the control is removed
                  // rather than left as a button that always fails.
                  onPressed: purchasable ? onAdd : null,
                  child: Text(purchasable ? '追加 / Add' : '不可 / N/A'),
                ),
            ],
          ),
        ),
      ),
    );
  }
}
