import 'dart:async';

import 'package:api_client/api_client.dart';
import 'package:domain_models/domain_models.dart';

/// Catalogue paging state.
///
/// Three behaviours are deliberate and tested:
/// - a **page failure never wipes the pages already loaded** (the atlas's
///   "a broken response must not destroy the existing model");
/// - a **category change replaces the list** — the products of the previous
///   category are not part of the new query, and neither is its cursor;
/// - the channel is always sent, so the server can withhold content this client
///   is not allowed to see.
class CatalogController with ChangeNotifierLike {
  CatalogController(this._api);

  final CommerceApiClient _api;

  List<ProductSummary> _items = const [];
  String? _cursor;
  String? _categoryId;
  bool _loading = false;
  bool _loadingMore = false;
  bool _hasMore = false;
  ApiProblem? _error;

  /// Id of the newest catalogue query.
  ///
  /// A response whose id is stale is dropped on arrival: a slow request for
  /// the previous category must not overwrite the category the shopper has
  /// picked in the meantime.
  int _queryId = 0;

  List<ProductSummary> get items => _items;
  bool get isLoading => _loading;
  bool get isLoadingMore => _loadingMore;
  bool get hasMore => _hasMore;
  ApiProblem? get error => _error;
  bool get isEmpty => _items.isEmpty && !_loading;

  /// The category the catalogue is filtered by; `null` means "everything".
  String? get categoryId => _categoryId;

  /// Loads the first page of the current filter.
  ///
  /// [refresh] drops the cursor and reloads from the top (pull to refresh).
  Future<void> load({bool refresh = false}) async {
    if (_loading || _loadingMore) return;
    if (refresh) {
      _cursor = null;
      _hasMore = false;
    }
    await _fetchFirstPage();
  }

  /// Filters the catalogue by [categoryId] and reloads it.
  ///
  /// `null` clears the filter. The list is **replaced**, never appended to:
  /// the products of the previous category are not part of this query, and
  /// the cursor belongs to the old one. A request still in flight for the
  /// previous category is superseded — its response is dropped when it lands.
  Future<void> selectCategory(String? categoryId) async {
    // Re-selecting the category that is already shown is a no-op, unless
    // there is nothing on screen to show (an error, or an empty category),
    // in which case the tap is the shopper's retry.
    if (categoryId == _categoryId && _items.isNotEmpty && !_loading) return;
    _categoryId = categoryId;
    _cursor = null;
    _hasMore = false;
    // The products on screen belong to the previous category. Keeping them
    // under the new selection would show the shopper exactly what the filter
    // excludes, so they go and a failed load leaves the error surface
    // behind instead of a stale list.
    _items = const [];
    await _fetchFirstPage();
  }

  /// Fetches the first page for the current filter and replaces the list.
  Future<void> _fetchFirstPage() async {
    final id = ++_queryId;
    _loading = true;
    _loadingMore = false;
    _error = null;
    notify();
    try {
      final page = await _api.listProducts(limit: 20, categoryId: _categoryId);
      if (id != _queryId) return;
      _items = page.items;
      _cursor = page.nextCursor;
      _hasMore = page.hasMore;
    } on ApiProblem catch (p) {
      // Keep whatever is already on screen.
      if (id != _queryId) return;
      _error = p;
    } finally {
      if (id == _queryId) {
        _loading = false;
        notify();
      }
    }
  }

  /// Loads the next page and appends it to the list.
  ///
  /// The category filter travels with the cursor: a page fetched without it
  /// would be a different result set than the one on screen.
  Future<void> loadMore() async {
    if (!_hasMore || _loadingMore || _loading) return;
    final id = ++_queryId;
    _loadingMore = true;
    notify();
    try {
      final page = await _api.listProducts(
        cursor: _cursor,
        limit: 20,
        categoryId: _categoryId,
      );
      if (id != _queryId) return;
      _items = [..._items, ...page.items];
      _cursor = page.nextCursor;
      _hasMore = page.hasMore;
      _error = null;
    } on ApiProblem catch (p) {
      // Keep whatever is already on screen.
      if (id != _queryId) return;
      _error = p;
    } finally {
      if (id == _queryId) {
        _loadingMore = false;
        notify();
      }
    }
  }

  Future<ProductDetail?> detail(String productId) async {
    try {
      return await _api.productDetail(productId);
    } on ApiProblem catch (p) {
      _error = p;
      notify();
      return null;
    }
  }
}

/// Minimal change-notification contract so this package has no Flutter
/// dependency and can be unit-tested on the Dart VM.
mixin ChangeNotifierLike {
  final List<void Function()> _listeners = [];

  void addListener(void Function() l) => _listeners.add(l);

  void removeListener(void Function() l) => _listeners.remove(l);

  void notify() {
    for (final l in List.of(_listeners)) {
      l();
    }
  }

  void dispose() => _listeners.clear();
}
