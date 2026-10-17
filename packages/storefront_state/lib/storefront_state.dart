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
export 'src/environment.dart';
