import 'package:design_system/design_system.dart';
import 'package:domain_models/domain_models.dart';
import 'package:flutter/material.dart';

/// A catalogue card. Shared verbatim by App, Web and LINE so the three
/// channels cannot drift visually.
///
/// The composition follows the reference storefront: image on top, then a
/// small title, then the price row with the original price struck through.
/// There is no card border, no shadow and no coloured badge — the whitespace
/// around the image is what separates one product from the next, which is
/// what makes a grid of a dozen items look like a shop rather than a table.
class ProductCard extends StatelessWidget {
  const ProductCard({
    super.key,
    required this.product,
    required this.onOpen,
    this.onAdd,
    this.imageUrl,
    this.showQuickAdd = true,
  });

  final ProductSummary product;
  final VoidCallback onOpen;
  final VoidCallback? onAdd;
  final String? imageUrl;
  final bool showQuickAdd;

  @override
  Widget build(BuildContext context) {
    final purchasable = product.availability != Availability.outOfStock;
    final isLowStock = product.availability == Availability.lowStock;
    final theme = Theme.of(context);

    return InkWell(
      onTap: onOpen,
      borderRadius: BorderRadius.circular(0),
      child: Column(
        crossAxisAlignment: CrossAxisAlignment.start,
        mainAxisSize: MainAxisSize.min,
        children: [
          // 1:1, the way the reference store photographs a garment. The
          // height is set by the grid, not by the card, so the photo and the
          // text below it always line up across a row.
          AspectRatio(
            aspectRatio: 1,
            child: _ProductImage(
              product: product,
              imageUrl: imageUrl,
            ),
          ),
          const SizedBox(height: DokSpace.md),
          Text(
            product.title,
            maxLines: 2,
            overflow: TextOverflow.ellipsis,
            style: theme.textTheme.titleLarge, // 14px serif, was titleSmall
          ),
          const SizedBox(height: DokSpace.xs),
          PriceLabel(money: product.price, showStrikethrough: true),
          const SizedBox(height: DokSpace.xs),
          AvailabilityBadge(
            availability: product.availability,
            dense: true,
            useWarmthForLowStock: isLowStock, // blush warmth for low stock
          ),
          if (onAdd != null && showQuickAdd) ...[
            const SizedBox(height: DokSpace.md),
            SizedBox(
              width: double.infinity,
              height: 48, // 44dp touch target
              child: FilledButton(
                onPressed: purchasable ? onAdd : null,
                child: Text(
                  purchasable
                      ? 'カートに追加'
                      : '在庫切れ / Out of stock',
                ),
              ),
            ),
          ],
        ],
      ),
    );
  }
}

/// The product photograph, or a typographic stand-in.
///
/// The catalogue carries no images yet, so the placeholder is not a grey
/// rectangle: it is a warm tonal panel with the product's initial letters
/// set large. That reads as a deliberate brand treatment instead of a
/// missing asset.
class _ProductImage extends StatelessWidget {
  const _ProductImage({required this.product, this.imageUrl});

  final ProductSummary product;
  final String? imageUrl;

  @override
  Widget build(BuildContext context) {
    final url = imageUrl;
    if (url != null && url.isNotEmpty) {
      return Image.network(
        url,
        fit: BoxFit.cover,
        width: double.infinity,
        height: double.infinity,
        errorBuilder: (context, error, stack) => _Placeholder(product: product),
      );
    }
    return _Placeholder(product: product);
  }
}

class _Placeholder extends StatelessWidget {
  const _Placeholder({required this.product});

  final ProductSummary product;

  @override
  Widget build(BuildContext context) {
    // A stable hue per product, derived from its id, so the grid does not
    // look like one repeated swatch.
    final seed = product.productId.codeUnits.fold<int>(0, (a, b) => a + b);
    final hue = (seed % 360).toDouble();
    // Use the warm accent palette instead of random hues
    final base = HSLColor.fromAHSL(1, hue, 0.12, 0.95).toColor();
    final ink = HSLColor.fromAHSL(1, hue, 0.25, 0.4).toColor();

    final initials = product.title
        .split(RegExp(r'[\s　/・]+'))
        .where((w) => w.isNotEmpty)
        .take(2)
        .map((w) => w.characters.first)
        .join();

    return Container(
      width: double.infinity,
      height: double.infinity,
      color: base,
      alignment: Alignment.center,
      child: Column(
        mainAxisSize: MainAxisSize.min,
        children: [
          Text(
            initials,
            style: TextStyle(
              fontFamily: DokTheme.displayFamily,
              fontSize: 40,
              height: 1,
              fontWeight: FontWeight.w300,
              letterSpacing: 1.2,
              color: ink.withValues(alpha: 0.55),
            ),
          ),
          const SizedBox(height: DokSpace.sm),
          Container(width: 28, height: 1, color: ink.withValues(alpha: 0.35)),
        ],
      ),
    );
  }
}
