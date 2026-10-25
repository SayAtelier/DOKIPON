import 'package:api_client/api_client.dart';
import 'package:design_system/design_system.dart';
import 'package:domain_models/domain_models.dart';
import 'package:flutter/material.dart';
import 'package:flutter_test/flutter_test.dart';
import 'package:storefront_state/storefront_state.dart';

/// The account sheet has to be dismissible by a control, not only by tapping
/// the barrier.
///
/// The UAT found it on the live deployment: the journey opened the account
/// sheet and then waited for a "閉じる / Close" button to close it — the
/// button the cart drawer has, and the account sheet did not have. The
/// journey timed out there, every run.
///
/// The sheet was dismissible by tapping the background, so a shopper with a
/// mouse could get out of it. A shopper who cannot tap the background — with
/// a keyboard, or with a screen reader — had no way out of it at all.
void main() {
  testWidgets('the account sheet has a close button, and it closes the sheet',
      (tester) async {
    final api = _FakeStorefront();

    await tester.pumpWidget(
      MaterialApp(
        theme: DokTheme.light(),
        home: StorefrontApp(
          environment: Environment(
            apiBaseUri: Uri.parse('https://example.test'),
            channel: StoreChannel.web,
          ),
          api: api,
        ),
      ),
    );
    await tester.pumpAndSettle();

    // Open the account sheet, the way a shopper does.
    await tester.tap(find.bySemanticsLabel('アカウント / Account'));
    await tester.pumpAndSettle();
    expect(find.text('セッション / Session: ゲスト / Guest'), findsOneWidget);

    // The sheet can be closed by a control — the same one the cart drawer
    // has, and the same label, so the two sheets are closed the same way.
    final close = find.byTooltip('閉じる / Close');
    expect(close, findsOneWidget, reason: 'the account sheet has no close button');

    await tester.tap(close);
    await tester.pumpAndSettle();
    expect(find.text('セッション / Session: ゲスト / Guest'), findsNothing);
  });
}

/// A storefront that answers from memory, so no test ever touches the network.
class _FakeStorefront extends CommerceApiClient {
  _FakeStorefront()
      : super(
          baseUri: Uri.parse('https://example.test'),
          channel: StoreChannel.web,
        );

  @override
  Future<ProductPage> listProducts({
    String? cursor,
    int limit = 20,
    String locale = 'ja',
    String? categoryId,
  }) async {
    return const ProductPage(items: []);
  }

  @override
  Future<Session> guestSession() async {
    return Session.guest('sess-test', channel: 'web');
  }
}
