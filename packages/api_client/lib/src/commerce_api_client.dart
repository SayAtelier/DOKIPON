import 'dart:async';
import 'dart:convert';

import 'package:domain_models/domain_models.dart';
import 'package:http/http.dart' as http;

import 'idempotency_key.dart';

/// Talks to the BFF. Injectable [httpClient] so tests never touch the network.
class CommerceApiClient {
  CommerceApiClient({
    required this.baseUri,
    required this.channel,
    http.Client? httpClient,
    IdempotencyKeyFactory? keys,
  })  : _http = httpClient ?? http.Client(),
        _keys = keys ?? IdempotencyKeyFactory();

  final Uri baseUri;
  final StoreChannel channel;
  final http.Client _http;
  final IdempotencyKeyFactory _keys;

  /// The current session. `null` before boot; guest or line-user after.
  Session? session;

  Map<String, String> _headers({bool json = true}) => {
        if (json) 'content-type': 'application/json',
        'accept': 'application/json',
        // The server filters content by channel; sending it is required, not cosmetic.
        'x-channel': channel.header,
        if (session != null) 'x-session-id': session!.sessionId,
      };

  Uri _uri(String path, [Map<String, String>? query]) =>
      baseUri.replace(path: path, queryParameters: query);

  Future<Map<String, dynamic>> _decode(http.Response res) async {
    final body = res.bodyBytes.isEmpty
        ? <String, dynamic>{}
        : jsonDecode(utf8.decode(res.bodyBytes)) as Map<String, dynamic>;

    if (res.statusCode >= 400) {
      // RFC 9457 problem body, with a defensive fallback for a proxy that
      // returned HTML instead.
      try {
        throw ApiProblem.fromJson(body);
      } on FormatException {
        throw ApiProblem(
          type: 'about:blank',
          title: 'Unexpected response',
          status: res.statusCode,
          code: 'client.malformed_error',
          detail: 'The server returned a non-problem error body.',
          traceId: res.headers['x-correlation-id'] ?? '',
          retryable: res.statusCode >= 500,
        );
      }
    }
    return body;
  }

  Future<Map<String, dynamic>> _get(
    String path, [
    Map<String, String>? query,
  ]) async {
    try {
      final res = await _http.get(_uri(path, query), headers: _headers(json: false));
      return await _decode(res);
    } on http.ClientException {
      throw ApiProblem.offline;
    }
  }

  Future<Map<String, dynamic>> _write(
    String method,
    String path, {
    Map<String, dynamic>? body,
    Map<String, String>? extraHeaders,
  }) async {
    try {
      final req = http.Request(method, _uri(path))
        ..headers.addAll(_headers())
        ..headers.addAll(extraHeaders ?? {});
      if (body != null) req.body = jsonEncode(body);
      final streamed = await _http.send(req);
      return await _decode(await http.Response.fromStream(streamed));
    } on http.ClientException {
      throw ApiProblem.offline;
    }
  }

  // ------------------------------------------------------------- storefront

  Future<StorefrontConfig> fetchStorefront() async {
    final json = await _get('/v1/storefront');
    return StorefrontConfig.fromJson(json, channel);
  }

  // ---------------------------------------------------------------- catalog

  /// Lists the catalogue.
  ///
  /// [categoryId] is the category filter the contract declares on this route
  /// (`contracts/openapi.yaml` -> `GET /v1/products`); the server filters on
  /// it. It is omitted from the query string when `null`, which asks for the
  /// whole catalogue.
  Future<ProductPage> listProducts({
    String? cursor,
    int limit = 20,
    String locale = 'ja',
    String? categoryId,
  }) async {
    final json = await _get('/v1/products', {
      'cursor': ?cursor,
      'limit': '$limit',
      'locale': locale,
      'categoryId': ?categoryId,
      'channel': channel.header,
    });
    return ProductPage.fromJson(json);
  }

  Future<ProductDetail> productDetail(String productId) async {
    final json = await _get('/v1/products/$productId');
    return ProductDetail.fromJson(json);
  }

  // ------------------------------------------------------------------- cart

  Future<Cart> getCart(String cartId) async =>
      Cart.fromJson(await _get('/v1/carts/$cartId'));

  /// Optimistic-concurrency mutation. The server bumps `version` on success;
  /// a 409 means the caller's view is stale and it must refetch, not overwrite.
  Future<Cart> updateCartItem({
    required String cartId,
    required String sku,
    required int quantity,
    required int expectedVersion,
    String operation = 'cart.update',
  }) async {
    final json = await _write(
      'PATCH',
      '/v1/carts/$cartId/items/$sku',
      body: {
        'quantity': quantity,
        'expectedVersion': expectedVersion,
      },
      extraHeaders: {'idempotency-key': _keys.keyFor(operation)},
    );
    return Cart.fromJson(json);
  }

  // --------------------------------------------------------------- checkout

  /// Builds the checkout request body.
  ///
  /// The field names live here and nowhere else, because they are the one
  /// place where a client can silently disagree with the server. The
  /// contract (`contracts/openapi.yaml` -> `CheckoutRequest`) says
  /// `paymentMethodRef` and `unit_amount_minor`; the order service accepts
  /// exactly that, and the server re-prices every line from the catalogue
  /// regardless of what is sent here.
  ///
  /// [lines] are the cart's lines. The `unit_amount_minor` is the cart's
  /// snapshot price — a display hint the server discards.
  static Map<String, dynamic> buildCheckoutPayload({
    required List<CartLine> lines,
    required String paymentMethodRef,
  }) {
    return {
      'lines': [
        for (final line in lines)
          {
            'sku': line.sku,
            'quantity': line.quantity,
            'unit_amount_minor': line.unitAmountSnapshot,
            'currency': line.snapshotMoney.currency.code,
          },
      ],
      'paymentMethodRef': paymentMethodRef,
    };
  }

  /// Submits a checkout. [operation] should identify the *logical* checkout
  /// (e.g. `checkout.cart-1`) so a transport retry replays rather than re-charges.
  Future<CheckoutSnapshot> submitCheckout({
    required String operation,
    required Map<String, dynamic> payload,
  }) async {
    final json = await _write(
      'POST',
      '/v1/checkouts',
      body: payload,
      extraHeaders: {'idempotency-key': _keys.keyFor(operation)},
    );
    return CheckoutSnapshot.fromJson(json);
  }

  /// Reads authoritative checkout status. This is the **only** safe action while
  /// a checkout is `payment_unknown` — re-submitting risks a double charge.
  Future<CheckoutSnapshot> checkoutStatus(String checkoutId) async =>
      CheckoutSnapshot.fromJson(await _get('/v1/checkouts/$checkoutId'));

  Future<List<Map<String, dynamic>>> listOrders() async {
    final json = await _get('/v1/orders');
    return ((json['items'] as List?) ?? const [])
        .map((e) => Map<String, dynamic>.from(e as Map))
        .toList(growable: false);
  }

  Future<Map<String, dynamic>> orderDetail(String orderId) async =>
      _get('/v1/orders/$orderId');

  // ---------------------------------------------------------------- identity

  /// Requests a guest session — the LIFF external-browser / not-logged-in path.
  Future<Session> guestSession() async {
    final json = await _write(
      'POST',
      '/v1/sessions/guest',
      body: {'channel': channel.header},
    );
    session = Session.fromJson(json);
    return session!;
  }

  /// Exchanges a LINE `id_token` for a short-lived session.
  ///
  /// The token is sent once and never stored by this client. Everything after
  /// this point uses the opaque `sessionId`.
  Future<Session> lineSession({
    required String idToken,
    String? nonce,
  }) async {
    final json = await _write(
      'POST',
      '/v1/sessions/line',
      body: {
        'id_token': idToken,
        'nonce': ?nonce,
        'channel': channel.header,
      },
    );
    session = Session.fromJson(json);
    return session!;
  }

  /// Discards client-side session state. The token was never kept, so there is
  /// nothing sensitive to purge beyond the identifier.
  void signOut() {
    session = null;
    _keys.clear();
  }

  void close() => _http.close();
}
