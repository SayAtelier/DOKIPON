import 'package:meta/meta.dart';

/// RFC 9457 Problem Details as extended by the server contract:
/// `{type, title, status, code, detail, traceId, retryable}`.
///
/// Clients branch on [code] and [retryable], never on the HTTP status alone.
/// In particular a `retryable: false` problem must never trigger a retry loop.
@immutable
class ApiProblem implements Exception {
  const ApiProblem({
    required this.type,
    required this.title,
    required this.status,
    required this.code,
    required this.detail,
    required this.traceId,
    required this.retryable,
  });

  final String type;
  final String title;
  final int status;
  final String code;
  final String detail;
  final String traceId;
  final bool retryable;

  /// True when the checkout must be re-polled rather than retried by resubmission.
  ///
  /// `payment.unknown` in particular means "the provider did not answer" —
  /// resubmitting a checkout could charge the customer twice.
  bool get isUnresolvedPayment => code.startsWith('payment.unknown') || code.startsWith('checkout.pending');

  /// Version-conflict on a cart mutation: the client should refetch and reapply.
  bool get isVersionConflict => code == 'cart.version_conflict';

  /// The Idempotency-Key was reused with a different payload. Retrying with the
  /// same key is wrong; a fresh key and a fresh read are required.
  bool get isIdempotencyConflict => code.contains('idempotency');

  /// The LINE channel is not available right now (SDK load failure, external
  /// browser). The rest of the storefront still works.
  bool get isLineUnavailable => code.startsWith('line.');

  /// A guest session cannot check out; a real LINE login is required.
  bool get requiresLogin => status == 401;

  /// The signed-in user does not own this resource.
  bool get isForbidden => status == 403;

  static const ApiProblem offline = ApiProblem(
    type: 'client.offline',
    title: 'Network unavailable',
    status: 0,
    code: 'client.offline',
    detail: 'The storefront could not reach the API.',
    traceId: '',
    retryable: true,
  );

  factory ApiProblem.fromJson(Map<String, dynamic> json) => ApiProblem(
        type: json['type'] as String? ?? 'about:blank',
        title: json['title'] as String? ?? 'Error',
        status: (json['status'] as num?)?.toInt() ?? 0,
        code: json['code'] as String? ?? 'unknown',
        detail: json['detail'] as String? ?? '',
        traceId: json['traceId'] as String? ?? '',
        retryable: json['retryable'] as bool? ?? false,
      );

  @override
  String toString() => 'ApiProblem($status $code: $detail)';
}

/// Whether a client session is authenticated, and through which channel.
enum SessionKind { guest, lineUser }

/// A server-issued session. The client stores only this identifier.
///
/// **A LIFF access token or id_token must never be persisted here** — it is
/// exchanged server-side for this short-lived session and then discarded.
@immutable
class Session {
  const Session({
    required this.sessionId,
    required this.kind,
    required this.expiresAtMs,
    this.channel = 'line',
  });

  final String sessionId;
  final SessionKind kind;
  final int expiresAtMs;
  final String channel;

  bool get isGuest => kind == SessionKind.guest;

  /// A guest may browse but not check out — the server enforces this too, and
  /// the UI must not offer a checkout button it knows will fail.
  bool get canCheckout => kind == SessionKind.lineUser;

  bool isValidAt(int nowMs) => !isGuest && expiresAtMs > nowMs;

  factory Session.guest(String sessionId) => Session(
        sessionId: sessionId,
        kind: SessionKind.guest,
        expiresAtMs: 0,
      );

  factory Session.fromJson(Map<String, dynamic> json) => Session(
        sessionId: json['sessionId'] as String? ?? '',
        kind: (json['kind'] as String?) == 'line_user'
            ? SessionKind.lineUser
            : SessionKind.guest,
        expiresAtMs: (json['expires_at_ms'] as num?)?.toInt() ??
            (json['expiresAtMs'] as num?)?.toInt() ??
            0,
        channel: json['channel'] as String? ?? 'line',
      );

  Map<String, dynamic> toJson() => {
        'sessionId': sessionId,
        'kind': kind == SessionKind.lineUser ? 'line_user' : 'guest',
        'expiresAtMs': expiresAtMs,
        'channel': channel,
      };
}
