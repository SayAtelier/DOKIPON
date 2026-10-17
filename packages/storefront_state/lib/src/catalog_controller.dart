import 'dart:async';

import 'package:api_client/api_client.dart';
import 'package:domain_models/domain_models.dart';

/// Catalogue paging state.
///
/// Two behaviours are deliberate and tested:
/// - a **page failure never wipes the pages already loaded** (the atlas's
///   "a broken response must not destroy the existing model");
/// - the channel is always sent, so the server can withhold content this client
///   is not allowed to see.
class CatalogController with ChangeNotifierLike {
  CatalogController(this._api);

  final CommerceApiClient _api;

  List<ProductSummary> _items = const [];
  String? _cursor;
  bool _loading = false;
  bool _loadingMore = false;
  bool _hasMore = false;
  ApiProblem? _error;

  List<ProductSummary> get items => _items;
  bool get isLoading => _loading;
  bool get isLoadingMore => _loadingMore;
  bool get hasMore => _hasMore;
  ApiProblem? get error => _error;
  bool get isEmpty => _items.isEmpty && !_loading;

  Future<void> load({bool refresh = false}) async {
    if (_loading || _loadingMore) return;
    if (refresh) {
      _cursor = null;
      _hasMore = false;
    }
    _loading = true;
    _error = null;
    notify();
    try {
      final page = await _api.listProducts(limit: 20);
      _items = page.items;
      _cursor = page.nextCursor;
      _hasMore = page.hasMore;
    } on ApiProblem catch (p) {
      // Keep whatever is already on screen.
      _error = p;
    } finally {
      _loading = false;
      notify();
    }
  }

  Future<void> loadMore() async {
    if (!_hasMore || _loadingMore || _loading) return;
    _loadingMore = true;
    notify();
    try {
      final page = await _api.listProducts(cursor: _cursor, limit: 20);
      _items = [..._items, ...page.items];
      _cursor = page.nextCursor;
      _hasMore = page.hasMore;
      _error = null;
    } on ApiProblem catch (p) {
      _error = p;
    } finally {
      _loadingMore = false;
      notify();
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
