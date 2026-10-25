/*
@cypher schema=1 source_sha256=2d060e019b32cffa7669a24e6db8b6c6b4f9a20cd0b58b8ad7efc10f90359a7b
MERGE (self:File {path:"packages/api_client/lib/src/admin_api_client.dart"})
MERGE (client:Symbol {id:"packages/api_client/lib/src/admin_api_client.dart::AdminApiClient", kind:"class"})
MERGE (login:Symbol {id:"packages/api_client/lib/src/admin_api_client.dart::AdminApiClient.login", kind:"method"})
MERGE (orders:Symbol {id:"packages/api_client/lib/src/admin_api_client.dart::AdminApiClient.listOrders", kind:"method"})
MERGE (detail:Symbol {id:"packages/api_client/lib/src/admin_api_client.dart::AdminApiClient.orderDetail", kind:"method"})
MERGE (reconcile:Symbol {id:"packages/api_client/lib/src/admin_api_client.dart::AdminApiClient.reconcileOrder", kind:"method"})
MERGE (catalog:Symbol {id:"packages/api_client/lib/src/admin_api_client.dart::AdminApiClient.listCatalog", kind:"method"})
MERGE (request:Symbol {id:"packages/api_client/lib/src/admin_api_client.dart::AdminApiClient._request", kind:"method"})
MERGE (decode:Symbol {id:"packages/api_client/lib/src/admin_api_client.dart::AdminApiClient._decode", kind:"method"})
MERGE (self)-[:DEFINES]->(client)
MERGE (client)-[:HAS_METHOD]->(login)
MERGE (client)-[:HAS_METHOD]->(orders)
MERGE (client)-[:HAS_METHOD]->(detail)
MERGE (client)-[:HAS_METHOD]->(reconcile)
MERGE (client)-[:HAS_METHOD]->(catalog)
MERGE (client)-[:HAS_METHOD]->(request)
MERGE (client)-[:HAS_METHOD]->(decode)
MERGE (login)-[:CALLS]->(request)
MERGE (orders)-[:CALLS]->(request)
MERGE (detail)-[:CALLS]->(request)
MERGE (reconcile)-[:CALLS]->(request)
MERGE (catalog)-[:CALLS]->(request)
MERGE (request)-[:CALLS]->(decode)
@endcypher
*/
import 'dart:convert';

import 'package:domain_models/domain_models.dart';
import 'package:http/http.dart' as http;

import 'admin_models.dart';

/// HTTP boundary for commerce administration. The bearer token stays in memory.
class AdminApiClient {
  AdminApiClient({required this.baseUri, http.Client? httpClient})
      : _http = httpClient ?? http.Client();

  // Paths mirror contracts/openapi.yaml `paths:` entries and the
  // commerce-admin service routes exactly. An earlier draft guessed
  // /v1/admin/auth/login and /v1/admin/orders, which the server does not serve:
  // the admin facade mounts everything under /admin/v1.
  static const loginPath = '/admin/v1/sessions';
  static const ordersPath = '/admin/v1/orders';
  static const catalogPath = '/admin/v1/products';
  static const inventoryPath = '/admin/v1/inventory';

  final Uri baseUri;
  final http.Client _http;
  String? _bearerToken;

  String? get bearerToken => _bearerToken;

  Future<void> login({required String username, required String password}) async {
    final response = await _request(
      'POST',
      _uri(loginPath),
      body: {'username': username, 'password': password},
      authenticated: false,
    );
    final token = response['accessToken'] ??
        response['access_token'] ??
        response['token'];
    if (token is! String || token.isEmpty) {
      throw const ApiProblem(
        type: 'client.malformed_response',
        title: 'Unexpected response',
        status: 0,
        code: 'client.malformed_response',
        detail: 'The admin service did not return an access token.',
        traceId: '',
        retryable: false,
      );
    }
    _bearerToken = token;
  }

  Future<List<AdminOrderSummary>> listOrders() async {
    final json = await _request('GET', _uri(ordersPath));
    final raw = json['items'] ?? json['orders'];
    return _parse(() => _maps(raw)
        .map(AdminOrderSummary.fromJson)
        .toList(growable: false));
  }

  Future<AdminOrderDetail> orderDetail(String orderId) async {
    final json = await _request(
      'GET',
      _orderUri(orderId),
    );
    return _parse(() => AdminOrderDetail.fromJson(json));
  }

  Future<void> reconcileOrder({
    required String orderId,
    required bool authorized,
  }) async {
    await _request(
      'POST',
      _orderUri(orderId, reconcile: true),
      body: {'authorized': authorized},
    );
  }

  Future<List<AdminCatalogProduct>> listCatalog() async {
    final json = await _request('GET', _uri(catalogPath));
    final raw = json['items'] ?? json['products'];
    return _parse(() => _maps(raw)
        .map(AdminCatalogProduct.fromJson)
        .toList(growable: false));
  }

  void clearBearerToken() => _bearerToken = null;

  Uri _uri(String path) => baseUri.replace(path: path);

  /// The order's own path.
  ///
  /// It is built from [ordersPath] the way every other path is, and for two
  /// reasons that both cost a real screen:
  ///
  ///   - the shape. `['v1', 'admin', 'orders', …]` is the **old draft's**
  ///     shape — the one the comment above says the server does not serve.
  ///     The facade mounts everything under `/admin/v1`.
  ///   - the leading slash. `Uri.replace(pathSegments: …)` builds a path with
  ///     **no leading slash**, so the browser resolves it against the
  ///     console's own URL: the console is at `/admin/`, and
  ///     `v1/admin/orders/ord-000001` becomes
  ///     `/admin/v1/admin/orders/ord-000001` — a 404 with an empty body,
  ///     and an order detail screen that says "Error" and offers a Retry
  ///     that cannot work.
  ///
  /// The reconcile path had the same two faults, and it is the one an
  /// operator uses to settle a payment they cannot read.
  Uri _orderUri(String orderId, {bool reconcile = false}) =>
      _uri('$ordersPath/$orderId${reconcile ? '/reconcile' : ''}');

  Future<Map<String, dynamic>> _request(
    String method,
    Uri uri, {
    Map<String, dynamic>? body,
    bool authenticated = true,
  }) async {
    try {
      final request = http.Request(method, uri)
        ..headers['accept'] = 'application/json'
        ..headers['content-type'] = 'application/json';
      if (authenticated && _bearerToken != null) {
        request.headers['authorization'] = 'Bearer $_bearerToken';
      }
      if (body != null) request.body = jsonEncode(body);
      final response = await http.Response.fromStream(await _http.send(request));
      return _decode(response);
    } on http.ClientException {
      throw const ApiProblem(
        type: 'client.offline',
        title: 'Admin service unavailable',
        status: 0,
        code: 'client.offline',
        detail: 'Could not reach the admin service. Check the connection and retry.',
        traceId: '',
        retryable: true,
      );
    }
  }

  Map<String, dynamic> _decode(http.Response response) {
    Map<String, dynamic> json;
    try {
      final decoded = response.bodyBytes.isEmpty
          ? <String, dynamic>{}
          : jsonDecode(utf8.decode(response.bodyBytes));
      json = decoded is Map
          ? Map<String, dynamic>.from(decoded)
          : <String, dynamic>{};
    } on FormatException {
      json = <String, dynamic>{};
    }

    if (response.statusCode >= 400) {
      final problem = ApiProblem.fromJson(json);
      throw ApiProblem(
        type: problem.type,
        title: problem.title,
        status: response.statusCode,
        code: problem.code,
        detail: problem.detail,
        traceId: problem.traceId,
        retryable: problem.retryable,
      );
    }
    if (response.bodyBytes.isNotEmpty && json.isEmpty) {
      throw const ApiProblem(
        type: 'client.malformed_response',
        title: 'Unexpected response',
        status: 0,
        code: 'client.malformed_response',
        detail: 'The admin service returned an invalid JSON object.',
        traceId: '',
        retryable: false,
      );
    }
    return json;
  }

  List<Map<String, dynamic>> _maps(dynamic value) {
    if (value is! List) {
      throw const FormatException('Expected an item list in the response.');
    }
    return value.map((item) {
      if (item is! Map) {
        throw const FormatException('Expected an object in the response list.');
      }
      return Map<String, dynamic>.from(item);
    }).toList(growable: false);
  }

  T _parse<T>(T Function() parse) {
    try {
      return parse();
    } on FormatException {
      throw _malformedResponse();
    } on ArgumentError {
      throw _malformedResponse();
    } on TypeError {
      throw _malformedResponse();
    }
  }

  ApiProblem _malformedResponse() => const ApiProblem(
        type: 'client.malformed_response',
        title: 'Unexpected response',
        status: 0,
        code: 'client.malformed_response',
        detail: 'The admin service returned invalid order or catalog data.',
        traceId: '',
        retryable: false,
      );

  void close() => _http.close();
}
