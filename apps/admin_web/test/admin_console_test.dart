/*
@cypher schema=1 source_sha256=f26c8fad7434b6848b3e11e5525fd47a7105e1bedf31839dd3b91c1a1262e1ac
MERGE (self:File {path:"apps/admin_web/test/admin_console_test.dart"})
MERGE (visibility:Symbol {id:"apps/admin_web/test/admin_console_test.dart::reconcile_actions_only_appear_for_payment_unknown", kind:"test"})
MERGE (auth401:Symbol {id:"apps/admin_web/test/admin_console_test.dart::unauthorized_orders_return_operator_to_login", kind:"test"})
MERGE (auth403:Symbol {id:"apps/admin_web/test/admin_console_test.dart::forbidden_orders_show_insufficient_role", kind:"test"})
MERGE (actions:Symbol {id:"apps/admin_web/lib/widgets/reconcile_actions.dart::ReconcileActions", kind:"class"})
MERGE (app:Symbol {id:"apps/admin_web/lib/admin_console_app.dart::AdminConsoleApp", kind:"class"})
MERGE (controller:Symbol {id:"packages/storefront_state/lib/src/admin_console_controller.dart::AdminConsoleController", kind:"class"})
MERGE (self)-[:DEFINES]->(visibility)
MERGE (self)-[:DEFINES]->(auth401)
MERGE (self)-[:DEFINES]->(auth403)
MERGE (visibility)-[:TESTS]->(actions)
MERGE (auth401)-[:TESTS]->(app)
MERGE (auth401)-[:TESTS]->(controller)
MERGE (auth403)-[:TESTS]->(app)
MERGE (auth403)-[:TESTS]->(controller)
@endcypher
*/
import 'package:admin_web/admin_console_app.dart';
import 'package:api_client/api_client.dart';
import 'package:domain_models/domain_models.dart';
import 'package:flutter/material.dart';
import 'package:flutter_test/flutter_test.dart';
import 'package:storefront_state/storefront_state.dart';

import 'package:admin_web/widgets/reconcile_actions.dart';

void main() {
  testWidgets('reconcile actions appear only for payment_unknown', (tester) async {
    for (final status in CheckoutStatus.values) {
      await tester.pumpWidget(
        MaterialApp(
          home: Scaffold(
            body: ReconcileActions(
              status: status,
              isWorking: false,
              onMarkAuthorized: () {},
              onMarkDeclined: () {},
            ),
          ),
        ),
      );

      final expected = status == CheckoutStatus.paymentUnknown;
      expect(find.text('Mark authorized'), expected ? findsOneWidget : findsNothing);
      expect(find.text('Mark declined'), expected ? findsOneWidget : findsNothing);
    }
  });

  testWidgets('401 clears the session and returns the operator to login',
      (tester) async {
    final api = _FakeAdminApiClient(
      orderProblem: const ApiProblem(
        type: 'about:blank',
        title: 'Unauthorized',
        status: 401,
        code: 'auth.expired',
        detail: 'The session has expired.',
        traceId: '',
        retryable: false,
      ),
    );
    final controller = AdminConsoleController(api);
    addTearDown(controller.dispose);

    await _signIn(tester, controller);

    expect(find.text('Commerce administration'), findsOneWidget);
    expect(find.text('Your session expired. Sign in again to continue.'),
        findsOneWidget);
    expect(controller.isAuthenticated, isFalse);
    expect(api.hasToken, isFalse);
  });

  testWidgets('403 keeps the session and shows insufficient role',
      (tester) async {
    final api = _FakeAdminApiClient(
      orderProblem: const ApiProblem(
        type: 'about:blank',
        title: 'Forbidden',
        status: 403,
        code: 'auth.insufficient_role',
        detail: 'This operator cannot list orders.',
        traceId: '',
        retryable: false,
      ),
    );
    final controller = AdminConsoleController(api);
    addTearDown(controller.dispose);

    await _signIn(tester, controller);

    expect(find.text('Orders', skipOffstage: false), findsWidgets);
    expect(find.textContaining('Insufficient role.'), findsOneWidget);
    expect(controller.isAuthenticated, isTrue);
    expect(api.hasToken, isTrue);
  });
}

Future<void> _signIn(
  WidgetTester tester,
  AdminConsoleController controller,
) async {
  await tester.pumpWidget(AdminConsoleApp(controller: controller));
  await tester.enterText(find.byType(TextFormField).at(0), 'operator');
  await tester.enterText(find.byType(TextFormField).at(1), 'password');
  await tester.tap(find.text('Sign in'));
  await tester.pumpAndSettle();
}

class _FakeAdminApiClient extends AdminApiClient {
  _FakeAdminApiClient({required this.orderProblem})
      : super(baseUri: Uri.parse('http://127.0.0.1:8080'));

  final ApiProblem orderProblem;
  bool hasToken = false;

  @override
  Future<void> login({required String username, required String password}) async {
    hasToken = true;
  }

  @override
  void clearBearerToken() {
    hasToken = false;
  }

  @override
  Future<List<AdminOrderSummary>> listOrders() async => throw orderProblem;
}
