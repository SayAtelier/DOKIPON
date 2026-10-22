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
    this.aspectRatio = 0.78,
  });

  final ProductSummary product;
  final VoidCallback onOpen;
  final VoidCallback? onAdd;

  /// Product photography. Absent in the current catalogue, in which case a
  /// typographic placeholder stands in rather than a grey box.
  final String? imageUrl;

  /// Taller than wide, the way a garment is photographed.
  final double aspectRatio;

  @override
  Widget build(BuildContext context) {
    final purchasable = product.availability != Availability.outOfStock;
    final theme = Theme.of(context);

    return InkWell(
      onTap: onOpen,
      child: Column(
        crossAxisAlignment: CrossAxisAlignment.start,
        mainAxisSize: MainAxisSize.min,
        children: [
          AspectRatio(
            aspectRatio: aspectRatio,
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
            style: theme.textTheme.titleSmall,
          ),
          const SizedBox(height: DokSpace.xs),
          PriceLabel(money: product.price),
          const SizedBox(height: DokSpace.xs),
          AvailabilityBadge(availability: product.availability, dense: true),
          if (onAdd != null) ...[
            const SizedBox(height: DokSpace.md),
            SizedBox(
              width: double.infinity,
              child: FilledButton(
                // Out-of-stock is not purchasable, so the control is removed
                // rather than left as a button that always fails.
                onPressed: purchasable ? onAdd : null,
                child: Text(purchasable ? 'カートに追加' : '在庫切れ'),
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
    final base = HSLColor.fromAHSL(1, hue, 0.16, 0.93).toColor();
    final ink = HSLColor.fromAHSL(1, hue, 0.28, 0.45).toColor();

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
