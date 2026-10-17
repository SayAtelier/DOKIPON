import 'package:flutter/material.dart';

import '../tokens.dart';

enum BannerTone { info, warning, danger, success }

/// A dismissible inline message.
///
/// Used for the degraded-channel notices (guest mode, LINE SDK unavailable,
/// external browser) as well as for checkout outcomes. The icon + text pairing
/// means tone is never carried by colour alone.
class StatusBanner extends StatelessWidget {
  const StatusBanner({
    super.key,
    required this.message,
    this.tone = BannerTone.info,
    this.actionLabel,
    this.onAction,
  });

  final String message;
  final BannerTone tone;
  final String? actionLabel;
  final VoidCallback? onAction;

  static IconData iconFor(BannerTone tone) => switch (tone) {
        BannerTone.info => Icons.info_outline,
        BannerTone.warning => Icons.warning_amber_outlined,
        BannerTone.danger => Icons.error_outline,
        BannerTone.success => Icons.check_circle_outline,
      };

  static Color colorFor(BannerTone tone) => switch (tone) {
        BannerTone.info => GemColors.info,
        BannerTone.warning => GemColors.warning,
        BannerTone.danger => GemColors.danger,
        BannerTone.success => GemColors.success,
      };

  @override
  Widget build(BuildContext context) {
    final color = colorFor(tone);
    return Semantics(
      liveRegion: true,
      child: Container(
        width: double.infinity,
        padding: const EdgeInsets.all(GemSpace.md),
        decoration: BoxDecoration(
          color: color.withValues(alpha: 0.08),
          borderRadius: const BorderRadius.all(GemRadius.md),
          border: Border.all(color: color.withValues(alpha: 0.30)),
        ),
        child: Row(
          crossAxisAlignment: CrossAxisAlignment.start,
          children: [
            Icon(iconFor(tone), color: color, size: 20),
            const SizedBox(width: GemSpace.sm),
            Expanded(
              child: Text(
                message,
                style: Theme.of(context)
                    .textTheme
                    .bodyMedium
                    ?.copyWith(color: GemColors.ink),
              ),
            ),
            if (actionLabel != null && onAction != null) ...[
              const SizedBox(width: GemSpace.sm),
              TextButton(
                onPressed: onAction,
                child: Text(actionLabel!),
              ),
            ],
          ],
        ),
      ),
    );
  }
}
