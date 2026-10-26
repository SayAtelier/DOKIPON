import 'package:api_client/api_client.dart';
import 'package:domain_models/domain_models.dart';

import 'catalog_controller.dart';

/// Boot-time identity resolution.
///
/// The ordering matters and is the whole point of the class:
/// 1. Ask LIFF (via the JS bridge) what environment we are in.
/// 2. Exchange a LINE `id_token` for a short-lived server session — once, and
///    the token is never stored.
/// 3. If that is impossible for any reason, fall back to a **guest session**,
///    which can browse but cannot check out.
///
/// It never loops. A failed LIFF login produces a guest session, not a redirect
/// cycle — the atlas's LIFF-002/003/004/005/006 cases.
class SessionController with ChangeNotifierLike {
  SessionController({
    required this.api,
    this.liff = const LiffBridge(),
  });

  final CommerceApiClient api;
  final LiffBridge liff;

  Session? _session;
  LiffContext _liffState = const LiffContext(
    ready: false,
    isInClient: false,
    isLoggedIn: false,
  );
  bool _booting = true;
  String? _notice;

  Session? get session => _session;

  /// The bridge is an implementation detail; callers read [liffState].
  LiffContext get liffState => _liffState;
  bool get isBooting => _booting;

  /// A user-facing explanation for a degraded state, or null.
  String? get notice => _notice;

  bool get canCheckout => _session?.canCheckout ?? false;
  bool get isGuest => _session?.isGuest ?? true;

  /// Runs once at app start. Always terminates.
  Future<void> boot() async {
    _booting = true;
    notify();
    try {
      _liffState = liff.read();

      if (_liffState.canRequestIdToken) {
        final token = _liffState.idToken;
        if (token != null && token.isNotEmpty) {
          try {
            // The token leaves the client here and is never written down.
            _session = await api.lineSession(idToken: token);
            _notice = null;
            return;
          } on ApiProblem catch (p) {
            // LIFF-006: revoked or rejected. Degrade to guest, do not retry.
            _notice = _messageFor(p);
          }
        }
      } else {
        _notice = _noticeForLiffState();
      }

      // The session may still be good. The server remembers it in a cookie the
      // browser sends on its own, so a reload does not have to mean a new
      // shopper: the same session means the same cart and the same orders.
      //
      // A session that is gone, expired or revoked answers with a failure,
      // and all three are the same thing here — there is nothing to continue,
      // so a new one is minted below.
      try {
        _session = await api.resumeSession();
        return;
      } on ApiProblem catch (_) {
        // No session to continue.
      }

      // Guest fallback — always succeeds, never loops.
      _session = await api.guestSession();
    } on ApiProblem catch (_) {
      _notice =
          'ストアに接続できませんでした / Could not reach the storefront.';
    } finally {
      _booting = false;
      notify();
    }
  }

  String _messageFor(ApiProblem p) {
    if (p.isForbidden) {
      return 'この操作にはログインが必要です / Sign-in is required for this action.';
    }
    return 'LINE 連携に失敗したため、ゲストとして閲覧しています / '
        'LINE sign-in failed; browsing as a guest.';
  }

  String? _noticeForLiffState() {
    if (_liffState.isSdkUnavailable) {
      // LIFF-003
      return 'LINE 機能を読み込めませんでした。ブラウズは可能です / '
          'LINE features unavailable; browsing still works.';
    }
    if (_liffState.isNotConfigured) {
      // LIFF-002
      return null; // dev build: not a problem worth showing
    }
    if (_liffState.isExternalBrowser) {
      // LIFF-004
      return 'LINE アプリ内で開くと greener な機能が使えます / '
          'Open inside LINE for the full experience.';
    }
    if (_liffState.needsLogin) {
      // LIFF-005
      return 'ログインすると購入できます / Sign in to check out.';
    }
    return null;
  }

  void clearNotice() {
    _notice = null;
    notify();
  }
}
