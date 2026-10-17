import 'dart:js_interop';

/// What the LIFF SDK reported. Only booleans and the token string ever leave
/// this boundary — no profile object, no display name, no picture URL.
class LiffContext {
  const LiffContext({
    required this.ready,
    required this.isInClient,
    required this.isLoggedIn,
    this.idToken,
    this.errorCode,
  });

  /// SDK loaded and `liff.init` succeeded.
  final bool ready;

  /// Running inside the LINE app WebView (false in an external browser).
  final bool isInClient;

  /// The user has granted access to the LINE account.
  final bool isLoggedIn;

  /// A one-shot `id_token`. Exchange it for a session immediately; never store it.
  final String? idToken;

  final String? errorCode;

  /// LIFF-002: the LIFF ID was never configured into this build.
  bool get isNotConfigured => ready && errorCode == 'LIFF_ID_NOT_CONFIGURED';

  /// LIFF-003: the SDK could not be fetched (offline, CSP, network).
  bool get isSdkUnavailable => !ready;

  /// LIFF-004: opened in Chrome/Safari rather than inside LINE.
  bool get isExternalBrowser => ready && !isInClient;

  /// LIFF-005: inside LINE but the user is not logged in.
  bool get needsLogin => ready && isInClient && !isLoggedIn;

  /// Can the storefront function here? Everything except an outright SDK
  /// failure degrades to plain Flutter Web — catalogue, cart and order screens
  /// do not depend on LIFF being available.
  bool get storefrontUsable => !isSdkUnavailable;

  /// Can the user reach checkout here? Only a verified in-client LINE session.
  bool get canRequestIdToken => ready && isInClient && isLoggedIn;

  /// A guest context: browsing works, checkout does not.
  bool get isGuestOnly => !canRequestIdToken;
}

/// The state object published by `web/liff_bridge.js` as
/// `window.__gemarcLiff`.
@JS('__gemarcLiff')
external _LiffJs? get _liff;

extension type _LiffJs(JSObject _) implements JSObject {
  external bool? get ready;
  external bool? get isInClient;
  external bool? get isLoggedIn;
  external String? get idToken;
  external String? get errorCode;
}

/// Dart-side view of the JS bridge.
///
/// The bridge is the *only* place the LIFF SDK is touched, which keeps the SDK
/// out of the Dart type system and makes the token's lifetime obvious: it is
/// read once, sent to the identity service, and never persisted.
///
/// On non-web platforms `window.__gemarcLiff` is absent, so [read] returns a
/// not-ready context and the app degrades to plain Flutter Web behaviour —
/// which is exactly what the Android/iOS builds want.
class LiffBridge {
  const LiffBridge();

  LiffContext read() {
    final state = _liff;
    if (state == null) {
      return const LiffContext(
        ready: false,
        isInClient: false,
        isLoggedIn: false,
        errorCode: 'BRIDGE_ABSENT',
      );
    }
    return LiffContext(
      ready: state.ready ?? false,
      isInClient: state.isInClient ?? false,
      isLoggedIn: state.isLoggedIn ?? false,
      idToken: state.idToken,
      errorCode: state.errorCode,
    );
  }
}
