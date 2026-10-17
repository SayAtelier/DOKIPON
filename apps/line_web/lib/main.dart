// LINE MINI App build.
//
// A LINE MINI App is a web app running inside the LINE WebView, so this is a
// Flutter Web target with the LIFF JS bridge (`web/liff_bridge.js`).
//
// Build:
//   flutter build web --dart-define=GEMARC_CHANNEL=line \
//     --dart-define=GEMARC_LIFF_ID=<public LIFF id> \
//     --dart-define=GEMARC_API_BASE=https://api.example.com
//
// The LIFF Channel Secret is NEVER passed to this build.

import 'package:api_client/api_client.dart';
import 'package:design_system/design_system.dart';
import 'package:storefront_state/storefront_state.dart';
import 'package:flutter/widgets.dart';

/// line_web
void main() {
  WidgetsFlutterBinding.ensureInitialized();
  final environment = Environment.fromDefines();
  final api = CommerceApiClient(
    baseUri: environment.apiBaseUri,
    channel: environment.channel,
  );
  runApp(StorefrontApp(environment: environment, api: api));
}
