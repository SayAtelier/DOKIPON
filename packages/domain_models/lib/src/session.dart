import 'package:domain_models/domain_models.dart';

/// The end-client surface a request belongs to.
///
/// The server filters catalogue content per channel, so this is not cosmetic:
/// a product that is not eligible for LINE must never be requested from LINE.
enum StoreChannel { web, app, line }

extension StoreChannelWire on StoreChannel {
  String get header => switch (this) {
        StoreChannel.web => 'web',
        StoreChannel.app => 'app',
        StoreChannel.line => 'line',
      };
}

/// Capability set the server reported for this client at boot.
class StorefrontConfig {
  const StorefrontConfig({
    required this.channel,
    required this.statusPollIntervalMs,
    required this.maxPollAttempts,
    required this.nativeShare,
    required this.pushNotifications,
    required this.externalBrowserOpen,
  });

  final StoreChannel channel;
  final int statusPollIntervalMs;
  final int maxPollAttempts;
  final bool nativeShare;
  final bool pushNotifications;
  final bool externalBrowserOpen;

  /// Fallbacks used when the storefront call itself fails (offline boot).
  /// LINE gets the most conservative set.
  factory StorefrontConfig.fallback(StoreChannel channel) => StorefrontConfig(
        channel: channel,
        statusPollIntervalMs: 1500,
        maxPollAttempts: 40,
        nativeShare: channel != StoreChannel.line,
        pushNotifications: channel != StoreChannel.line,
        externalBrowserOpen: channel == StoreChannel.line,
      );

  factory StorefrontConfig.fromJson(
    Map<String, dynamic> json,
    StoreChannel channel,
  ) {
    final checkout = Map<String, dynamic>.from(
      json['checkout'] as Map? ?? const {},
    );
    final caps = Map<String, dynamic>.from(
      json['capabilities'] as Map? ?? const {},
    );
    return StorefrontConfig(
      channel: channel,
      statusPollIntervalMs:
          (checkout['statusPollIntervalMs'] as num?)?.toInt() ?? 1500,
      maxPollAttempts: (checkout['maxPollAttempts'] as num?)?.toInt() ?? 40,
      nativeShare: caps['nativeShare'] as bool? ?? channel != StoreChannel.line,
      pushNotifications: caps['pushNotifications'] as bool? ??
          channel != StoreChannel.line,
      externalBrowserOpen: caps['externalBrowserOpen'] as bool? ??
          channel == StoreChannel.line,
    );
  }
}

/// A cart line as the client models it.
///
/// [unitAmountSnapshot] is display-only. The server recomputes every total from
/// the authoritative catalogue price at checkout, so tampering with this field
/// changes what the user *sees* and nothing about what they are *charged*.
class CartLine {
  const CartLine({
    required this.sku,
    required this.quantity,
    required this.unitAmountSnapshot,
  });

  final String sku;
  final int quantity;
  final int unitAmountSnapshot;

  Money get snapshotMoney =>
      Money.of(unitAmountSnapshot < 0 ? 0 : unitAmountSnapshot, Currency.jpy);

  factory CartLine.fromJson(Map<String, dynamic> json) => CartLine(
        sku: json['sku'] as String? ?? '',
        quantity: (json['quantity'] as num?)?.toInt() ?? 0,
        unitAmountSnapshot:
            (json['unit_amount_snapshot'] as num?)?.toInt() ??
                (json['unitAmountSnapshot'] as num?)?.toInt() ??
                0,
      );

  Map<String, dynamic> toJson() => {
        'sku': sku,
        'quantity': quantity,
        'unit_amount_snapshot': unitAmountSnapshot,
      };
}

/// The client-side cart. [version] drives optimistic concurrency: every
/// mutation sends the version it was based on, and a 409 means "refetch and
/// retry", never "overwrite".
class Cart {
  const Cart({
    required this.cartId,
    required this.version,
    required this.lines,
    this.ownerId,
  });

  final String cartId;
  final int version;
  final List<CartLine> lines;
  final String? ownerId;

  bool get isEmpty => lines.isEmpty;

  /// Sum of the *snapshots*. Shown as "your cart" total. The authoritative
  /// amount is always recomputed server-side at checkout.
  Money get snapshotSubtotal => lines.fold<Money>(
        Money.zero(Currency.jpy),
        (acc, line) => acc + line.snapshotMoney.multipliedBy(line.quantity),
      );

  factory Cart.fromJson(Map<String, dynamic> json) => Cart(
        cartId: json['cartId'] as String? ?? json['cart_id'] as String? ?? '',
        version: (json['version'] as num?)?.toInt() ?? 1,
        ownerId: json['owner_id'] as String? ?? json['ownerId'] as String?,
        lines: ((json['items'] as List?) ?? const [])
            .map((e) => CartLine.fromJson(Map<String, dynamic>.from(e as Map)))
            .toList(growable: false),
      );
}
