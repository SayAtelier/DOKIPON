// Android / iOS build of the DOKIPON storefront.
//
// Channel is compiled in as `app`; the server filters catalogue content
// accordingly and no LINE-specific behaviour is reachable from this build.

import 'package:api_client/api_client.dart';
import 'package:design_system/design_system.dart';
import 'package:storefront_state/storefront_state.dart';
import 'package:flutter/widgets.dart';

/// shop_app
void main() {
  WidgetsFlutterBinding.ensureInitialized();
  final environment = Environment.fromDefines();
  final api = CommerceApiClient(
    baseUri: environment.apiBaseUri,
    channel: environment.channel,
  );
  runApp(StorefrontApp(environment: environment, api: api));
}
