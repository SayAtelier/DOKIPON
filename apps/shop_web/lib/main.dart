// Web build of the GemArc Commerce storefront.
//
// Build:
//   flutter build web --dart-define=GEMARC_CHANNEL=web \
//     --dart-define=GEMARC_API_BASE=https://api.example.com

import 'package:api_client/api_client.dart';
import 'package:design_system/design_system.dart';
import 'package:storefront_state/storefront_state.dart';
import 'package:flutter/widgets.dart';

/// shop_web
void main() {
  WidgetsFlutterBinding.ensureInitialized();
  final environment = Environment.fromDefines();
  final api = CommerceApiClient(
    baseUri: environment.apiBaseUri,
    channel: environment.channel,
  );
  runApp(StorefrontApp(environment: environment, api: api));
}
