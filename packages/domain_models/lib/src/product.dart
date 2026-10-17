import 'package:meta/meta.dart';

import 'money.dart';

/// How a SKU relates to the warehouse. Only [StockPolicy.reserved] SKUs take
/// part in the inventory Saga step.
enum StockPolicy { unbounded, reserved, madeToOrder }

extension StockPolicyWire on StockPolicy {
  String get wire => switch (this) {
        StockPolicy.unbounded => 'unbounded',
        StockPolicy.reserved => 'reserved',
        StockPolicy.madeToOrder => 'made_to_order',
      };

  bool get needsInventory => this == StockPolicy.reserved;
}

/// Catalogue availability. `madeToOrder` is not a stock level.
enum Availability { inStock, lowStock, outOfStock, madeToOrder }

extension AvailabilityWire on Availability {
  String get wire => switch (this) {
        Availability.inStock => 'in_stock',
        Availability.lowStock => 'low_stock',
        Availability.outOfStock => 'out_of_stock',
        Availability.madeToOrder => 'made_to_order',
      };
}

/// Which end-client surfaces may show a product.
///
/// The LINE MINI App column is not decoration: the design atlas forbids
/// shipping policy-excluded content into that channel (LIFF-010), and the
/// server filters on this field.
@immutable
class ChannelEligibility {
  const ChannelEligibility({
    this.web = true,
    this.app = true,
    this.lineMiniApp = true,
  });

  final bool web;
  final bool app;
  final bool lineMiniApp;

  bool allows(String channel) => switch (channel) {
        'line' => lineMiniApp,
        'app' => app,
        _ => web,
      };

  factory ChannelEligibility.fromJson(Map<String, dynamic>? json) {
    if (json == null) return const ChannelEligibility();
    return ChannelEligibility(
      web: json['web'] as bool? ?? true,
      app: json['app'] as bool? ?? true,
      lineMiniApp: json['line_mini_app'] as bool? ?? true,
    );
  }

  @override
  bool operator ==(Object other) =>
      other is ChannelEligibility &&
      other.web == web &&
      other.app == app &&
      other.lineMiniApp == lineMiniApp;

  @override
  int get hashCode => Object.hash(web, app, lineMiniApp);
}

/// A list-page product.
@immutable
class ProductSummary {
  const ProductSummary({
    required this.productId,
    required this.sku,
    required this.title,
    required this.price,
    required this.availability,
  });

  final String productId;
  final String sku;

  /// Already localized by the server, with a ja→any fallback applied there.
  final String title;
  final Money price;
  final Availability availability;

  factory ProductSummary.fromJson(Map<String, dynamic> json) => ProductSummary(
        productId: json['productId'] as String? ?? '',
        sku: json['sku'] as String? ?? '',
        title: json['title'] as String? ?? '',
        price: Money.fromJson(
          Map<String, dynamic>.from(
            json['price'] as Map? ?? {'amountMinor': json['amountMinor'], 'currency': json['currency']},
          ),
        ),
        availability: _availability(json['availability'] as String?),
      );

  static Availability _availability(String? raw) => switch (raw) {
        'in_stock' => Availability.inStock,
        'low_stock' => Availability.lowStock,
        'out_of_stock' => Availability.outOfStock,
        'made_to_order' => Availability.madeToOrder,
        _ => Availability.inStock,
      };
}

/// Full product detail, including the authoritative price and version.
@immutable
class ProductDetail {
  const ProductDetail({
    required this.productId,
    required this.sku,
    required this.price,
    required this.version,
    required this.stockPolicy,
    required this.availability,
    required this.eligibility,
    this.title = '',
    this.titleI18n = const {},
  });

  final String productId;
  final String sku;
  final Money price;

  /// Server-side price version. A cart holding an older version must be
  /// re-priced at checkout rather than silently honoured.
  final int version;
  final StockPolicy stockPolicy;
  final Availability availability;
  final ChannelEligibility eligibility;
  final String title;
  final Map<String, String> titleI18n;

  /// Client-side localization fallback: requested locale → ja → first entry.
  String localizedTitle(String locale) =>
      titleI18n[locale] ?? titleI18n['ja'] ?? (titleI18n.isEmpty ? title : titleI18n.values.first);

  factory ProductDetail.fromJson(Map<String, dynamic> json) {
    final i18n = (json['titleI18n'] as Map?) ?? const {};
    return ProductDetail(
      productId: json['productId'] as String? ?? '',
      sku: json['sku'] as String? ?? '',
      price: Money.fromJson(Map<String, dynamic>.from(json['price'] as Map? ?? const {})),
      version: (json['version'] as num?)?.toInt() ?? 1,
      stockPolicy: switch (json['stockPolicy'] as String?) {
        'unbounded' => StockPolicy.unbounded,
        'made_to_order' => StockPolicy.madeToOrder,
        _ => StockPolicy.reserved,
      },
      availability: ProductSummary._availability(json['availability'] as String?),
      eligibility: ChannelEligibility.fromJson(
        json['channelEligibility'] as Map<String, dynamic>?,
      ),
      titleI18n: i18n.map((k, v) => MapEntry('$k', '$v')),
    );
  }
}

/// A cursor-paginated slice of products.
@immutable
class ProductPage {
  const ProductPage({required this.items, this.nextCursor});

  final List<ProductSummary> items;
  final String? nextCursor;

  bool get hasMore => nextCursor != null && nextCursor!.isNotEmpty;

  factory ProductPage.fromJson(Map<String, dynamic> json) => ProductPage(
        items: ((json['items'] as List?) ?? const [])
            .map((e) => ProductSummary.fromJson(Map<String, dynamic>.from(e as Map)))
            .toList(growable: false),
        nextCursor: json['next_cursor'] as String? ?? json['nextCursor'] as String?,
      );
}
