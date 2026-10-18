/// Non-web half of the LIFF bridge.
///
/// `window.__gemarcLiff` cannot exist off the web platform, so this always
/// reports an absent bridge. That is the truthful answer for VM tests, native
/// builds and CI: [LiffBridge.read] turns null into the documented
/// BRIDGE_ABSENT context, which is also what an Android/iOS build wants.
///
/// This file exists so `dart:js_interop` is never compiled into a non-web
/// build. Without it, `import 'package:api_client/api_client.dart'` fails in a
/// widget test with "Type 'JSObject' not found" before a single test runs.
library;

import 'liff_bridge.dart';

LiffContext? readLiffJsState() => null;
