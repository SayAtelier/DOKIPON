/*
@cypher schema=1 source_sha256=580a5cba0e08a7f6190fd9d02d3884d54b0627ce16cd0c663a2de78b0431d56a
MERGE (self:File {path:"packages/api_client/lib/src/admin_models.dart"})
MERGE (summary:Symbol {id:"packages/api_client/lib/src/admin_models.dart::AdminOrderSummary", kind:"class"})
MERGE (detail:Symbol {id:"packages/api_client/lib/src/admin_models.dart::AdminOrderDetail", kind:"class"})
MERGE (trail:Symbol {id:"packages/api_client/lib/src/admin_models.dart::AdminOrderTrailEntry", kind:"class"})
MERGE (product:Symbol {id:"packages/api_client/lib/src/admin_models.dart::AdminCatalogProduct", kind:"class"})
MERGE (money:Symbol {id:"packages/domain_models/lib/src/money.dart::Money", kind:"class"})
MERGE (checkout:Symbol {id:"packages/domain_models/lib/src/checkout.dart::CheckoutStatus", kind:"enum"})
MERGE (eligibility:Symbol {id:"packages/domain_models/lib/src/product.dart::ChannelEligibility", kind:"class"})
MERGE (domain:Symbol {id:"packages/domain_models/lib/domain_models.dart::domain_models", kind:"module"})
MERGE (self)-[:IMPORTS]->(domain)
MERGE (self)-[:DEFINES]->(summary)
MERGE (self)-[:DEFINES]->(detail)
MERGE (self)-[:DEFINES]->(trail)
MERGE (self)-[:DEFINES]->(product)
MERGE (summary)-[:USES_TYPE]->(money)
MERGE (summary)-[:USES_TYPE]->(checkout)
MERGE (detail)-[:USES_TYPE]->(money)
MERGE (detail)-[:USES_TYPE]->(checkout)
MERGE (product)-[:USES_TYPE]->(money)
MERGE (product)-[:USES_TYPE]->(eligibility)
@endcypher
*/
import 'package:domain_models/domain_models.dart';

/// A commerce order row exposed to authorized operators.
class AdminOrderSummary {
  const AdminOrderSummary({
    required this.orderId,
    required this.state,
    required this.total,
    this.createdAt,
  });

  final String orderId;
  final CheckoutStatus state;
  final Money total;
  final DateTime? createdAt;

  factory AdminOrderSummary.fromJson(Map<String, dynamic> json) =>
      AdminOrderSummary(
        orderId: _string(json, const ['orderId', 'order_id', 'id']),
        state: CheckoutStatus.parse(
          _optionalString(json, const ['state', 'status']),
        ),
        total: _money(json['total'] ?? json['amount'], fallback: json),
        createdAt: _dateTime(json, const ['createdAt', 'created_at']),
      );
}

/// Detailed order data, including its captured lines and state trail.
class AdminOrderDetail {
  const AdminOrderDetail({
    required this.orderId,
    required this.state,
    required this.total,
    required this.lines,
    required this.trail,
    this.createdAt,
  });

  final String orderId;
  final CheckoutStatus state;
  final Money total;
  final List<OrderLine> lines;
  final List<AdminOrderTrailEntry> trail;
  final DateTime? createdAt;

  factory AdminOrderDetail.fromJson(Map<String, dynamic> json) {
    final rawLines = json['lines'] ?? json['items'];
    final rawTrail = json['trail'] ?? json['timeline'];
    return AdminOrderDetail(
      orderId: _string(json, const ['orderId', 'order_id', 'id']),
      state: CheckoutStatus.parse(
        _optionalString(json, const ['state', 'status']),
      ),
      total: _money(json['total'] ?? json['amount'], fallback: json),
      createdAt: _dateTime(json, const ['createdAt', 'created_at']),
      lines: _mapList(rawLines).map(_orderLine).toList(growable: false),
      trail: _mapList(rawTrail)
          .map(AdminOrderTrailEntry.fromJson)
          .toList(growable: false),
    );
  }
}

/// One immutable state transition recorded by the commerce service.
class AdminOrderTrailEntry {
  const AdminOrderTrailEntry({
    required this.sequence,
    required this.event,
    required this.from,
    required this.to,
    this.occurredAt,
  });

  final int sequence;
  final String event;
  final CheckoutStatus from;
  final CheckoutStatus to;
  final DateTime? occurredAt;

  factory AdminOrderTrailEntry.fromJson(Map<String, dynamic> json) =>
      AdminOrderTrailEntry(
        sequence: _int(json['sequence']),
        event: _optionalString(json, const ['event']) ?? '',
        from: CheckoutStatus.parse(
          _optionalString(json, const ['fromState', 'from_state']),
        ),
        to: CheckoutStatus.parse(
          _optionalString(json, const ['toState', 'to_state']),
        ),
        occurredAt: _dateTime(json, const ['occurredAt', 'occurred_at']),
      );
}

/// Read-only product fields needed by the catalog operator screen.
class AdminCatalogProduct {
  const AdminCatalogProduct({
    required this.productId,
    required this.sku,
    required this.title,
    required this.price,
    required this.eligibility,
    required this.hasEligibility,
  });

  final String productId;
  final String sku;
  final String title;
  final Money price;
  final ChannelEligibility eligibility;
  final bool hasEligibility;

  factory AdminCatalogProduct.fromJson(Map<String, dynamic> json) {
    final rawEligibility = json['channelEligibility'] ??
        json['channel_eligibility'] ??
        json['eligibility'];
    final hasEligibility = rawEligibility is Map || rawEligibility is List;
    final eligibility = rawEligibility is Map
        ? ChannelEligibility.fromJson(
            Map<String, dynamic>.from(rawEligibility),
          )
        : _eligibilityFromList(rawEligibility);
    return AdminCatalogProduct(
      productId: _string(json, const ['productId', 'product_id', 'id']),
      sku: _string(json, const ['sku']),
      title: _string(json, const ['title', 'name']),
      price: _money(json['price'] ?? json['amount'], fallback: json),
      eligibility: eligibility,
      hasEligibility: hasEligibility,
    );
  }
}

List<Map<String, dynamic>> _mapList(dynamic value) => value is List
    ? value.map((item) {
        if (item is! Map) {
          throw const FormatException('Expected an object in the response list.');
        }
        return Map<String, dynamic>.from(item);
      }).toList(growable: false)
    : throw const FormatException('Expected a list in the response.');

OrderLine _orderLine(Map<String, dynamic> json) {
  final quantity = _int(json['quantity']);
  if (quantity < 0) {
    throw const FormatException('Line item quantity must not be negative.');
  }
  final rawPrice = json['unitAmount'] ??
      json['unit_amount'] ??
      json['unitPrice'] ??
      json['unit_price'];
  return OrderLine(
    sku: _string(json, const ['sku']),
    quantity: quantity,
    unitPrice: _money(rawPrice, fallback: json),
  );
}

Money _money(dynamic value, {required Map<String, dynamic> fallback}) {
  final json = value is Map ? Map<String, dynamic>.from(value) : const {};
  final rawAmount = json['amountMinor'] ??
      json['amount_minor'] ??
      json['amount'] ??
      (value is int || value is String
          ? value
          : fallback['amountMinor'] ??
              fallback['amount_minor'] ??
              fallback['amount']);
  final rawCurrency = json['currency'] ??
      json['currencyCode'] ??
      json['currency_code'] ??
      fallback['currency'] ??
      fallback['currencyCode'] ??
      fallback['currency_code'];
  if (rawAmount == null || rawCurrency is! String) {
    throw const FormatException('Money must include integer minor units and currency.');
  }
  return Money.of(_int(rawAmount), Currency.parse(rawCurrency));
}

ChannelEligibility _eligibilityFromList(dynamic value) {
  if (value is! List) return const ChannelEligibility();
  final channels = value.map((item) => '$item').toSet();
  return ChannelEligibility(
    web: channels.contains('web'),
    app: channels.contains('app'),
    lineMiniApp: channels.contains('line') || channels.contains('line_mini_app'),
  );
}

String _string(Map<String, dynamic> json, List<String> keys) =>
    _optionalString(json, keys) ?? '';

String? _optionalString(Map<String, dynamic> json, List<String> keys) {
  for (final key in keys) {
    final value = json[key];
    if (value is String) return value;
  }
  return null;
}

int _int(dynamic value) {
  if (value is int) return value;
  if (value is String) {
    final parsed = int.tryParse(value);
    if (parsed != null) return parsed;
  }
  throw const FormatException('Expected an integer value.');
}

DateTime? _dateTime(Map<String, dynamic> json, List<String> keys) {
  final value = _optionalString(json, keys);
  return value == null ? null : DateTime.tryParse(value);
}
