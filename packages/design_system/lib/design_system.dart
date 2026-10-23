/// Visual tokens and shared widgets for the DOKIPON clients.
///
/// Accessibility rules applied here rather than per screen:
/// - every interactive control keeps a 44dp minimum touch target (WCAG 2.2
///   target-size guidance), including when its visual box is smaller;
/// - status is never conveyed by colour alone — every badge carries a text
///   label, because `out_of_stock` and `low_stock` look identical to a
///   red/green colour-blind user otherwise;
/// - text scales with the platform text-size factor rather than being pinned.
library;

export 'src/tokens.dart';
export 'src/widgets/availability_badge.dart';
export 'src/widgets/cart_drawer.dart';
export 'src/widgets/price_label.dart';
export 'src/widgets/status_banner.dart';
export 'src/widgets/product_card.dart';
export 'src/widgets/storefront_app.dart';
