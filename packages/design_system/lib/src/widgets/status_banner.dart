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
        BannerTone.info => DokColors.info,
        BannerTone.warning => DokColors.warning,
        BannerTone.danger => DokColors.danger,
        BannerTone.success => DokColors.success,
      };

  @override
  Widget build(BuildContext context) {
    final color = colorFor(tone);
    return Semantics(
      liveRegion: true,
      child: Container(
        width: double.infinity,
        padding: const EdgeInsets.all(DokSpace.md),
        decoration: BoxDecoration(
          color: color.withValues(alpha: 0.06),
          border: Border(
            left: BorderSide(color: color, width: 2),
            top: const BorderSide(color: DokColors.border),
            right: const BorderSide(color: DokColors.border),
            bottom: const BorderSide(color: DokColors.border),
          ),
        ),
        child: Row(
          crossAxisAlignment: CrossAxisAlignment.start,
          children: [
            Icon(iconFor(tone), color: color, size: 18),
            const SizedBox(width: DokSpace.sm),
            Expanded(
              child: Text(
                message,
                style: Theme.of(context)
                    .textTheme
                    .bodyMedium
                    ?.copyWith(color: DokColors.ink),
              ),
            ),
            if (actionLabel != null && onAction != null) ...[
              const SizedBox(width: DokSpace.sm),
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
