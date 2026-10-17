import 'dart:math';

/// Generates and remembers `Idempotency-Key` values.
///
/// The important behaviour is *stickiness*: calling [keyFor] twice for the same
/// logical operation returns the same key, so a retry after a timeout is a
/// replay rather than a new operation.
class IdempotencyKeyFactory {
  IdempotencyKeyFactory({Random? random}) : _random = random ?? Random.secure();

  final Random _random;
  final Map<String, String> _issued = {};

  /// The key for `operation`. Repeated calls return the same value until
  /// [rotate] is called.
  String keyFor(String operation) =>
      _issued[operation] ??= _newKey();

  /// Discards the key for `operation` so the next [keyFor] mints a new one.
  /// Only call this after the operation has definitively succeeded, or after a
  /// `KeyReuseConflict` — never merely because a request timed out.
  void rotate(String operation) => _issued.remove(operation);

  /// Drops every remembered key, e.g. when the session changes.
  void clear() => _issued.clear();

  bool hasKeyFor(String operation) => _issued.containsKey(operation);

  String _newKey() {
    // RFC 4122 version 4 layout, formatted lowercase without braces.
    final bytes = List<int>.generate(16, (_) => _random.nextInt(256));
    bytes[6] = (bytes[6] & 0x0f) | 0x40; // version 4
    bytes[8] = (bytes[8] & 0x3f) | 0x80; // variant 1
    final hex = bytes.map((b) => b.toRadixString(16).padLeft(2, '0')).join();
    return '${hex.substring(0, 8)}-${hex.substring(8, 12)}-'
        '${hex.substring(12, 16)}-${hex.substring(16, 20)}-${hex.substring(20)}';
  }
}
