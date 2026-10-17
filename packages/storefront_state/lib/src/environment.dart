import 'package:domain_models/domain_models.dart';

/// Build-time configuration. Supplied by `--dart-define`, never hardcoded, so
/// the same source produces a LINE build and an App build.
class Environment {
  const Environment({
    required this.apiBaseUri,
    required this.channel,
    this.locale = 'ja',
    this.lineLiffIdPresent = false,
  });

  final Uri apiBaseUri;
  final StoreChannel channel;
  final String locale;

  /// Whether a LIFF ID was compiled into this build. Absent in dev builds and
  /// in the App builds, where the LINE channel does not apply.
  final bool lineLiffIdPresent;

  static const _defaultBase = String.fromEnvironment(
    'GEMARC_API_BASE',
    defaultValue: 'http://127.0.0.1:8080',
  );

  factory Environment.fromDefines() {
    const rawChannel = String.fromEnvironment('GEMARC_CHANNEL', defaultValue: 'web');
    const liffId = String.fromEnvironment('GEMARC_LIFF_ID');
    return Environment(
      apiBaseUri: Uri.parse(_defaultBase),
      channel: switch (rawChannel) {
        'line' => StoreChannel.line,
        'app' => StoreChannel.app,
        _ => StoreChannel.web,
      },
      lineLiffIdPresent: liffId.isNotEmpty,
    );
  }

  /// Development default. Never used in a release build.
  static Environment dev(StoreChannel channel) => Environment(
        apiBaseUri: Uri.parse('http://127.0.0.1:8080'),
        channel: channel,
      );
}
