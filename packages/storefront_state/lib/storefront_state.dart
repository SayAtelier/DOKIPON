/*
@cypher schema=1 source_sha256=5359ffa5ac8f045426771dbb1086e2ab5f99d7d59147592461d1cbc1765ae8a0
MERGE (self:File {path:"packages/storefront_state/lib/storefront_state.dart"})
MERGE (api:Symbol {id:"packages/storefront_state/lib/storefront_state.dart::storefront_state", kind:"module"})
MERGE (controller:Symbol {id:"packages/storefront_state/lib/src/admin_console_controller.dart::AdminConsoleController", kind:"class"})
MERGE (self)-[:DEFINES]->(api)
MERGE (self)-[:EXPOSES]->(controller)
@endcypher
*/
/// Presentation-agnostic storefront state.
///
/// The three clients differ only in *how* they boot and which channels exist,
/// never in what they show. That is the atlas's `FR-CORE-05`: Flutter Web and
/// the mobile/LIFF clients share one domain model. All screen logic lives here;
/// the apps supply the widgets.
///
/// Nothing in this package imports `dart:html`, `dart:io`, or a platform
/// channel, so it unit-tests identically on the VM.
library;

export 'src/catalog_controller.dart';
export 'src/checkout_controller.dart';
export 'src/session_controller.dart';
export 'src/admin_console_controller.dart';
export 'src/environment.dart';
