import 'package:flutter/material.dart';

/// Design tokens. The atlas's `UIUX_DESIGN_SPEC.md` asks for high-precision
/// tokens rather than ad-hoc values, so every spacing/colour decision lands
/// here.
class GemColors {
  const GemColors._();

  static const brand = Color(0xFF1F6FEB);
  static const brandDark = Color(0xFF144A96);

  /// LINE's brand green, used only inside the LINE MINI App build.
  static const line = Color(0xFF06C755);

  static const ink = Color(0xFF111827);
  static const inkMuted = Color(0xFF6B7280);
  static const surface = Color(0xFFFFFFFF);
  static const surfaceAlt = Color(0xFFF7F8FA);
  static const border = Color(0xFFE5E7EB);

  static const success = Color(0xFF047857);
  static const warning = Color(0xFFB45309);
  static const danger = Color(0xFFB91C1C);
  static const info = Color(0xFF1D4ED8);
}

/// A 4-pt spacing scale.
class GemSpace {
  const GemSpace._();

  static const xs = 4.0;
  static const sm = 8.0;
  static const md = 16.0;
  static const lg = 24.0;
  static const xl = 32.0;
}

/// Minimum interactive size. Never go below this.
const double kMinTouchTarget = 48.0;

class GemRadius {
  const GemRadius._();

  static const sm = Radius.circular(6);
  static const md = Radius.circular(10);
  static const lg = Radius.circular(16);
}

class GemTheme {
  const GemTheme._();

  static ThemeData light({Color seed = GemColors.brand}) {
    final scheme = ColorScheme.fromSeed(seedColor: seed);
    return ThemeData(
      useMaterial3: true,
      colorScheme: scheme,
      scaffoldBackgroundColor: GemColors.surfaceAlt,
      visualDensity: VisualDensity.standard,
      filledButtonTheme: FilledButtonThemeData(
        style: FilledButton.styleFrom(
          // WCAG 2.2 target size.
          minimumSize: const Size(kMinTouchTarget, kMinTouchTarget),
          shape: const RoundedRectangleBorder(
            borderRadius: BorderRadius.all(GemRadius.md),
          ),
        ),
      ),
      outlinedButtonTheme: OutlinedButtonThemeData(
        style: OutlinedButton.styleFrom(
          minimumSize: const Size(kMinTouchTarget, kMinTouchTarget),
          shape: const RoundedRectangleBorder(
            borderRadius: BorderRadius.all(GemRadius.md),
          ),
        ),
      ),
      textButtonTheme: TextButtonThemeData(
        style: TextButton.styleFrom(
          minimumSize: const Size(kMinTouchTarget, kMinTouchTarget),
        ),
      ),
      cardTheme: CardThemeData(
        elevation: 0,
        color: GemColors.surface,
        shape: RoundedRectangleBorder(
          borderRadius: const BorderRadius.all(GemRadius.lg),
          side: const BorderSide(color: GemColors.border),
        ),
      ),
      snackBarTheme: const SnackBarThemeData(
        behavior: SnackBarBehavior.floating,
      ),
    );
  }
}
