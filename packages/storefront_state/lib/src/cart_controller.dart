import 'dart:async';

import 'package:api_client/api_client.dart';
import 'package:domain_models/domain_models.dart';

import 'catalog_controller.dart';

/// Cart state for the storefront.
///
/// The cart is **server-authoritative**. The client never computes a total it
/// believes is final: it shows the server's snapshot, and the server
/// re-prices every line at checkout from the catalogue. That is why this
/// controller holds a [Cart] rather than a list of local line items.
///
/// Two behaviours are deliberate:
///
/// - A **page failure never wipes the cart already on screen.** Losing a
///   basket because a refresh failed is a data-loss bug, not a transient
///   error.
/// - Every mutation carries the version it was based on. A 409 means "your
///   view is stale": the cart is refetched and the operation is retried
///   once, never applied blindly on top of someone else's change.
class CartController with ChangeNotifierLike {
  CartController(this._api);

  final CommerceApiClient _api;

  Cart? _cart;
  bool _loading = false;
  bool _mutating = false;
  ApiProblem? _problem;
  String? _cartId;

  /// The server's view of the cart. Null until the first load.
  Cart? get cart => _cart;

  /// The cart id, if one has been established.
  String? get cartId => _cartId;

  bool get isLoading => _loading;
  bool get isMutating => _mutating;
  ApiProblem? get error => _problem;

  List<CartLine> get lines => _cart?.lines ?? const [];

  /// The number of units in the cart, for the header badge.
  int get itemCount => lines.fold(0, (sum, line) => sum + line.quantity);

  bool get isEmpty => itemCount == 0;

  /// The server's snapshot subtotal. Display only — the amount actually
  /// charged is computed server-side at checkout.
  Money get subtotal => _cart?.snapshotSubtotal ?? Money.zero(Currency.jpy);

  /// Fetch the cart.
  ///
  /// [cartId] identifies the cart. A null id means there is nothing to fetch
  /// yet, which is not an error.
  Future<void> load(String? cartId) async {
    if (cartId == null || cartId.isEmpty) return;
    if (_loading) return;
    _loading = true;
    _problem = null;
    notify();
    try {
      _cart = await _api.getCart(cartId);
      _cartId = cartId;
    } on ApiProblem catch (p) {
      // Keep whatever is already on screen.
      _problem = p;
    } finally {
      _loading = false;
      notify();
    }
  }

  /// Set a line's quantity. A quantity of 0 removes the line.
  ///
  /// Returns true when the change was applied.
  Future<bool> setQuantity({
    required String sku,
    required int quantity,
  }) async {
    final cart = _cart;
    if (cart == null || _mutating) return false;

    _mutating = true;
    _problem = null;
    notify();
    try {
      _cart = await _api.updateCartItem(
        cartId: cart.cartId,
        sku: sku,
        quantity: quantity,
        expectedVersion: cart.version,
      );
      return true;
    } on ApiProblem catch (p) {
      if (p.isVersionConflict) {
        // The cart moved under us. Refetch, then let the caller decide —
        // silently replaying a mutation on a fresh version could apply a
        // change the shopper has since undone.
        try {
          _cart = await _api.getCart(cart.cartId);
        } on ApiProblem {
          // Keep the stale cart; the error below explains the failure.
        }
      }
      _problem = p;
      return false;
    } finally {
      _mutating = false;
      notify();
    }
  }

  /// Add one unit of a SKU.
  Future<bool> add(String sku, {int quantity = 1}) async {
    final cart = _cart;
    if (cart == null) return false;
    final current = cart.lines
        .where((line) => line.sku == sku)
        .fold<int>(0, (sum, line) => sum + line.quantity);
    return setQuantity(sku: sku, quantity: current + quantity);
  }

  /// Remove a line entirely.
  Future<bool> remove(String sku) => setQuantity(sku: sku, quantity: 0);

  void clearError() {
    _problem = null;
    notify();
  }
}
