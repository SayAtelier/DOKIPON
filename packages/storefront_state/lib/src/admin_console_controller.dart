/*
@cypher schema=1 source_sha256=967609f3207eeb884c2adf15d202a2654b65cd091a4fffb3effe27de13bb9c88
MERGE (self:File {path:"packages/storefront_state/lib/src/admin_console_controller.dart"})
MERGE (controller:Symbol {id:"packages/storefront_state/lib/src/admin_console_controller.dart::AdminConsoleController", kind:"class"})
MERGE (login:Symbol {id:"packages/storefront_state/lib/src/admin_console_controller.dart::AdminConsoleController.signIn", kind:"method"})
MERGE (orders:Symbol {id:"packages/storefront_state/lib/src/admin_console_controller.dart::AdminConsoleController.loadOrders", kind:"method"})
MERGE (detail:Symbol {id:"packages/storefront_state/lib/src/admin_console_controller.dart::AdminConsoleController.loadOrderDetail", kind:"method"})
MERGE (reconcile:Symbol {id:"packages/storefront_state/lib/src/admin_console_controller.dart::AdminConsoleController.reconcile", kind:"method"})
MERGE (catalog:Symbol {id:"packages/storefront_state/lib/src/admin_console_controller.dart::AdminConsoleController.loadCatalog", kind:"method"})
MERGE (problem:Symbol {id:"packages/storefront_state/lib/src/admin_console_controller.dart::AdminConsoleController._problemMessage", kind:"method"})
MERGE (api:Symbol {id:"packages/api_client/lib/src/admin_api_client.dart::AdminApiClient", kind:"class"})
MERGE (self)-[:DEFINES]->(controller)
MERGE (controller)-[:HAS_METHOD]->(login)
MERGE (controller)-[:HAS_METHOD]->(orders)
MERGE (controller)-[:HAS_METHOD]->(detail)
MERGE (controller)-[:HAS_METHOD]->(reconcile)
MERGE (controller)-[:HAS_METHOD]->(catalog)
MERGE (controller)-[:HAS_METHOD]->(problem)
MERGE (controller)-[:USES_TYPE]->(api)
MERGE (login)-[:CALLS]->(problem)
MERGE (orders)-[:CALLS]->(problem)
MERGE (detail)-[:CALLS]->(problem)
MERGE (reconcile)-[:CALLS]->(problem)
MERGE (catalog)-[:CALLS]->(problem)
@endcypher
*/
import 'package:api_client/api_client.dart';
import 'package:domain_models/domain_models.dart';
import 'package:flutter/foundation.dart';

/// Admin screen state and explicit authentication-error handling.
class AdminConsoleController extends ChangeNotifier {
  AdminConsoleController(this._api);

  final AdminApiClient _api;

  bool _authenticated = false;
  bool _signingIn = false;
  bool _ordersLoading = false;
  bool _orderLoading = false;
  bool _catalogLoading = false;
  bool _reconciling = false;
  bool _ordersLoaded = false;
  bool _catalogLoaded = false;
  int _sessionEpoch = 0;
  int _ordersRequestId = 0;
  int _orderRequestId = 0;
  int _catalogRequestId = 0;
  String? _loginError;
  String? _sessionNotice;
  String? _ordersError;
  String? _orderError;
  String? _catalogError;
  CheckoutStatus? _orderFilter;
  String? _selectedOrderId;
  AdminOrderDetail? _selectedOrder;
  List<AdminOrderSummary> _orders = const [];
  List<AdminCatalogProduct> _catalog = const [];

  bool get isAuthenticated => _authenticated;
  bool get isSigningIn => _signingIn;
  bool get isOrdersLoading => _ordersLoading;
  bool get isOrderLoading => _orderLoading;
  bool get isCatalogLoading => _catalogLoading;
  bool get isReconciling => _reconciling;
  bool get hasLoadedOrders => _ordersLoaded;
  bool get hasLoadedCatalog => _catalogLoaded;
  String? get loginError => _loginError;
  String? get sessionNotice => _sessionNotice;
  String? get ordersError => _ordersError;
  String? get orderError => _orderError;
  String? get catalogError => _catalogError;
  CheckoutStatus? get orderFilter => _orderFilter;
  String? get selectedOrderId => _selectedOrderId;
  AdminOrderDetail? get selectedOrder => _selectedOrder;
  List<AdminOrderSummary> get orders => _orderFilter == null
      ? _orders
      : _orders.where((order) => order.state == _orderFilter).toList();
  List<AdminCatalogProduct> get catalog => _catalog;

  Future<void> signIn({
    required String username,
    required String password,
  }) async {
    if (_signingIn) return;
    _signingIn = true;
    _loginError = null;
    _sessionNotice = null;
    notifyListeners();
    try {
      await _api.login(username: username, password: password);
      _authenticated = true;
      _sessionEpoch++;
      await loadOrders();
    } on ApiProblem catch (problem) {
      _loginError = _problemMessage(problem, duringSignIn: true);
    } finally {
      _signingIn = false;
      notifyListeners();
    }
  }

  Future<void> loadOrders() async {
    if (!_authenticated || _ordersLoading) return;
    final requestId = ++_ordersRequestId;
    final sessionEpoch = _sessionEpoch;
    _ordersLoading = true;
    _ordersError = null;
    notifyListeners();
    try {
      final orders = await _api.listOrders();
      if (_isCurrent(requestId, _ordersRequestId, sessionEpoch)) {
        _orders = orders;
        _ordersLoaded = true;
      }
    } on ApiProblem catch (problem) {
      final message = _problemMessage(problem);
      if (_isCurrent(requestId, _ordersRequestId, sessionEpoch)) {
        _ordersError = message;
      }
    } finally {
      if (requestId == _ordersRequestId) {
        _ordersLoading = false;
        notifyListeners();
      }
    }
  }

  void setOrderFilter(CheckoutStatus? status) {
    _orderFilter = status;
    notifyListeners();
  }

  Future<void> loadOrderDetail(String orderId) async {
    if (!_authenticated) return;
    final requestId = ++_orderRequestId;
    final sessionEpoch = _sessionEpoch;
    _selectedOrderId = orderId;
    _selectedOrder = null;
    _orderLoading = true;
    _orderError = null;
    notifyListeners();
    try {
      final order = await _api.orderDetail(orderId);
      if (_isCurrent(requestId, _orderRequestId, sessionEpoch)) {
        _selectedOrder = order;
      }
    } on ApiProblem catch (problem) {
      final message = _problemMessage(problem);
      if (_isCurrent(requestId, _orderRequestId, sessionEpoch)) {
        _orderError = message;
      }
    } finally {
      if (requestId == _orderRequestId) {
        _orderLoading = false;
        notifyListeners();
      }
    }
  }

  Future<void> reconcile({required bool authorized}) async {
    final order = _selectedOrder;
    final orderId = _selectedOrderId;
    if (!_authenticated ||
        _reconciling ||
        order == null ||
        orderId == null ||
        order.state != CheckoutStatus.paymentUnknown) {
      return;
    }

    _reconciling = true;
    _orderError = null;
    notifyListeners();
    try {
      await _api.reconcileOrder(orderId: orderId, authorized: authorized);
      await loadOrderDetail(orderId);
      await loadOrders();
    } on ApiProblem catch (problem) {
      _orderError = _problemMessage(problem);
    } finally {
      _reconciling = false;
      notifyListeners();
    }
  }

  Future<void> loadCatalog() async {
    if (!_authenticated || _catalogLoading) return;
    final requestId = ++_catalogRequestId;
    final sessionEpoch = _sessionEpoch;
    _catalogLoading = true;
    _catalogError = null;
    notifyListeners();
    try {
      final catalog = await _api.listCatalog();
      if (_isCurrent(requestId, _catalogRequestId, sessionEpoch)) {
        _catalog = catalog;
        _catalogLoaded = true;
      }
    } on ApiProblem catch (problem) {
      final message = _problemMessage(problem);
      if (_isCurrent(requestId, _catalogRequestId, sessionEpoch)) {
        _catalogError = message;
      }
    } finally {
      if (requestId == _catalogRequestId) {
        _catalogLoading = false;
        notifyListeners();
      }
    }
  }

  void signOut() {
    _api.clearBearerToken();
    _authenticated = false;
    _sessionEpoch++;
    _cancelRequests();
    _clearProtectedData();
    _loginError = null;
    _sessionNotice = null;
    notifyListeners();
  }

  void _clearProtectedData() {
    _orders = const [];
    _catalog = const [];
    _selectedOrder = null;
    _selectedOrderId = null;
    _ordersLoaded = false;
    _catalogLoaded = false;
    _ordersError = null;
    _orderError = null;
    _catalogError = null;
  }

  void _cancelRequests() {
    _ordersRequestId++;
    _orderRequestId++;
    _catalogRequestId++;
    _ordersLoading = false;
    _orderLoading = false;
    _catalogLoading = false;
    _reconciling = false;
  }

  bool _isCurrent(int requestId, int currentId, int sessionEpoch) =>
      _authenticated && requestId == currentId && sessionEpoch == _sessionEpoch;

  void _invalidateSession() {
    _api.clearBearerToken();
    _authenticated = false;
    _sessionEpoch++;
    _cancelRequests();
    _clearProtectedData();
    _sessionNotice = 'Your session expired. Sign in again to continue.';
    notifyListeners();
  }

  String _problemMessage(ApiProblem problem, {bool duringSignIn = false}) {
    if (problem.status == 401) {
      _api.clearBearerToken();
      if (duringSignIn) return 'Invalid username or password.';
      _invalidateSession();
      return 'Your session expired. Sign in again to continue.';
    }
    if (problem.status == 403) {
      return 'Insufficient role. Contact an administrator if you need access.';
    }
    return problem.detail.isNotEmpty ? problem.detail : problem.title;
  }

  @override
  void dispose() {
    _api.close();
    super.dispose();
  }
}
