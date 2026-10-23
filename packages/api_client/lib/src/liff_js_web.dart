/// The state object published by `web/liff_bridge.js` as `window.__gemarcLiff`.
///
/// Web implementation. See `liff_js_stub.dart` for the non-web half.
library;

import 'dart:js_interop';

import 'liff_bridge.dart';

@JS('__gemarcLiff')
external _LiffJs? get _liff;

extension type _LiffJs(JSObject _) implements JSObject {
  external bool? get ready;
  external bool? get isInClient;
  external bool? get isLoggedIn;
  external String? get idToken;
  external String? get errorCode;
}

/// Reads the JS bridge into a plain Dart value.
///
/// A missing bridge is null, and an absent individual field becomes false, so
/// a partially initialised SDK cannot leak a null into a non-nullable bool.
LiffContext? readLiffJsState() {
  final state = _liff;
  if (state == null) {
    return null;
  }
  return LiffContext(
    ready: state.ready ?? false,
    isInClient: state.isInClient ?? false,
    isLoggedIn: state.isLoggedIn ?? false,
    idToken: state.idToken,
    errorCode: state.errorCode,
  );
}
