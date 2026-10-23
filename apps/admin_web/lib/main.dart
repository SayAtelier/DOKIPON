/*
@cypher schema=1 source_sha256=5c3fa4b8ce867d1d13fd94f6e4ffaca22b08f058d0039666147a02b514df1370
MERGE (self:File {path:"apps/admin_web/lib/main.dart"})
MERGE (entry:Symbol {id:"apps/admin_web/lib/main.dart::main", kind:"function"})
MERGE (environment:Symbol {id:"packages/storefront_state/lib/src/environment.dart::Environment", kind:"class"})
MERGE (client:Symbol {id:"packages/api_client/lib/src/admin_api_client.dart::AdminApiClient", kind:"class"})
MERGE (controller:Symbol {id:"packages/storefront_state/lib/src/admin_console_controller.dart::AdminConsoleController", kind:"class"})
MERGE (app:Symbol {id:"apps/admin_web/lib/admin_console_app.dart::AdminConsoleApp", kind:"class"})
MERGE (self)-[:DEFINES]->(entry)
MERGE (entry)-[:CALLS]->(environment)
MERGE (entry)-[:CALLS]->(client)
MERGE (entry)-[:CALLS]->(controller)
MERGE (entry)-[:CALLS]->(app)
@endcypher
*/
import 'package:api_client/api_client.dart';
import 'package:flutter/widgets.dart';
import 'package:storefront_state/storefront_state.dart';

import 'admin_console_app.dart';

void main() {
  WidgetsFlutterBinding.ensureInitialized();
  final environment = Environment.fromDefines();
  final api = AdminApiClient(baseUri: environment.apiBaseUri);
  runApp(AdminConsoleApp(controller: AdminConsoleController(api)));
}
