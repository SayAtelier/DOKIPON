import 'package:flutter/material.dart';

/// Design tokens for DOKIPON.
///
/// The visual direction is "quiet luxury": a near-monochrome palette, a
/// serif display face for headings, generous vertical rhythm, and a single
/// warm accent that carries every call to action.
///
/// The reference the storefront was modelled on (a Japanese lingerie
/// house) reads as first-tier because of what it does NOT do: no coloured
/// buttons, no gradients, no drop shadows on cards, no coloured "sale" pills.
/// It uses white space, a small type scale, one dark ink, and lets the
/// product imagery carry the colour. The tokens below encode that, so the
/// Flutter build lands in the same register rather than looking like a
/// Material demo.
///
/// Accessibility rules applied here rather than per screen:
/// - every interactive control keeps a 44dp minimum touch target (WCAG 2.2
///   target-size guidance);
/// - status is never conveyed by colour alone — every badge carries a text
///   label;
/// - text scales with the platform text-size factor rather than being pinned.
class DokColors {
  const DokColors._();

  /// The brand ink. Near-black with a warm cast, so it reads as "ink on
  /// paper" rather than the cold blue-black of a default theme.
  static const ink = Color(0xFF1A1A1A);

  /// Secondary text. Also near-black, not grey: a true grey at small sizes
  /// on white drops below the 4.5:1 contrast ratio that body copy needs.
  static const inkSoft = Color(0xFF55534F);

  /// The faintest tier — rules, dividers, disabled labels.
  static const inkFaint = Color(0xFF8C8880);

  static const surface = Color(0xFFFFFFFF);

  /// The page background. A barely-off-white, so the product grid does not
  /// sit on a hard white slab.
  static const canvas = Color(0xFFFAF9F7);

  /// The card / well colour — a touch warmer than the canvas.
  static const surfaceAlt = Color(0xFFF2EFEA);

  static const border = Color(0xFFE4DFD8);
  static const borderStrong = Color(0xFFCDC6BC);

  /// The accent: a deep rose. Used for the sale price, the "new" flag and
  /// the primary button — never for decoration.
  static const accent = Color(0xFF9E2A4B);
  static const accentSoft = Color(0xFFF7E7EC);

  /// Deep charcoal for the primary button. A near-black button reads as
  /// premium; a coloured one reads as promotional.
  static const button = Color(0xFF1A1A1A);
  static const onButton = Color(0xFFFFFFFF);

  static const success = Color(0xFF2F6B4F);
  static const warning = Color(0xFF9A6212);
  static const danger = Color(0xFFA32020);
  static const info = Color(0xFF2B5470);

  /// LINE's brand green, used only inside the LINE MINI App build.
  static const line = Color(0xFF06C755);
}

/// A 4-pt spacing scale, extended to the 8-pt rhythm a dense grid needs.
class DokSpace {
  const DokSpace._();

  static const xxs = 2.0;
  static const xs = 4.0;
  static const sm = 8.0;
  static const md = 12.0;
  static const lg = 16.0;
  static const xl = 24.0;
  static const xxl = 32.0;
  static const xxxl = 48.0;
  static const huge = 72.0;
}

/// Minimum interactive size. Never go below this.
const double kMinTouchTarget = 44.0;

/// The page's horizontal gutter. Wider on a large screen: a 4-column grid
/// at 1440px wants a wider margin than a 2-column grid on a phone.
class DokGutter {
  const DokGutter._();

  static double of(double width) {
    if (width >= 1200) return 48;
    if (width >= 900) return 32;
    if (width >= 600) return 24;
    return 16;
  }

  /// How many product columns the grid should have at this width.
  static int columnsFor(double width) {
    if (width >= 1400) return 4;
    if (width >= 1000) return 3;
    if (width >= 620) return 2;
    return 2;
  }
}

class DokRadius {
  const DokRadius._();

  static const none = Radius.circular(0);
  static const sm = Radius.circular(2);
  static const md = Radius.circular(4);
  static const lg = Radius.circular(8);
  static const pill = Radius.circular(999);
}

class DokTheme {
  const DokTheme._();

  /// The display face. A serif for headings is the single strongest signal
  /// that separates a fashion storefront from a catalogue — the reference
  /// uses one for its H1 and section titles.
  static const displayFamily = 'Georgia, "Times New Roman", serif';

  static ThemeData light({Color? seed}) {
    final scheme = ColorScheme.fromSeed(
      seedColor: seed ?? DokColors.accent,
      brightness: Brightness.light,
    ).copyWith(
      primary: DokColors.ink,
      onPrimary: DokColors.onButton,
      secondary: DokColors.accent,
      onSecondary: DokColors.onButton,
      surface: DokColors.surface,
      onSurface: DokColors.ink,
      error: DokColors.danger,
    );

    const serif = TextStyle(
      fontFamily: displayFamily,
      color: DokColors.ink,
      letterSpacing: 0.2,
    );

    return ThemeData(
      useMaterial3: true,
      colorScheme: scheme,
      scaffoldBackgroundColor: DokColors.canvas,
      fontFamily: null,
      // The reference store's body copy is small and dense; 14px at desktop
      // with a generous line height reads as considered rather than cramped.
      textTheme: const TextTheme(
        displayLarge: serif,
        displayMedium: serif,
        displaySmall: serif,
        headlineLarge: serif,
        headlineMedium: serif,
        headlineSmall: serif,
        titleLarge: serif,
        titleMedium: TextStyle(
          color: DokColors.ink,
          fontSize: 14,
          height: 1.45,
          fontWeight: FontWeight.w500,
        ),
        titleSmall: TextStyle(
          color: DokColors.ink,
          fontSize: 13,
          height: 1.4,
          fontWeight: FontWeight.w500,
        ),
        bodyLarge: TextStyle(
          color: DokColors.ink,
          fontSize: 14,
          height: 1.6,
        ),
        bodyMedium: TextStyle(
          color: DokColors.inkSoft,
          fontSize: 13,
          height: 1.6,
        ),
        bodySmall: TextStyle(
          color: DokColors.inkFaint,
          fontSize: 11,
          height: 1.5,
        ),
        labelLarge: TextStyle(
          color: DokColors.ink,
          fontSize: 13,
          fontWeight: FontWeight.w600,
          letterSpacing: 0.4,
        ),
        labelMedium: TextStyle(
          color: DokColors.inkSoft,
          fontSize: 11,
          letterSpacing: 0.6,
          fontWeight: FontWeight.w500,
        ),
        labelSmall: TextStyle(
          color: DokColors.inkFaint,
          fontSize: 10,
          letterSpacing: 0.8,
          fontWeight: FontWeight.w600,
        ),
      ),
      visualDensity: VisualDensity.standard,
      appBarTheme: const AppBarTheme(
        backgroundColor: DokColors.surface,
        surfaceTintColor: Colors.transparent,
        foregroundColor: DokColors.ink,
        elevation: 0,
        scrolledUnderElevation: 0,
        centerTitle: false,
      ),
      filledButtonTheme: FilledButtonThemeData(
        style: FilledButton.styleFrom(
          minimumSize: const Size(kMinTouchTarget, kMinTouchTarget),
          backgroundColor: DokColors.button,
          foregroundColor: DokColors.onButton,
          textStyle: const TextStyle(
            fontSize: 13,
            fontWeight: FontWeight.w600,
            letterSpacing: 0.8,
          ),
          shape: const RoundedRectangleBorder(
            borderRadius: BorderRadius.all(DokRadius.none),
          ),
        ),
      ),
      outlinedButtonTheme: OutlinedButtonThemeData(
        style: OutlinedButton.styleFrom(
          minimumSize: const Size(kMinTouchTarget, kMinTouchTarget),
          foregroundColor: DokColors.ink,
          side: const BorderSide(color: DokColors.borderStrong),
          textStyle: const TextStyle(
            fontSize: 13,
            fontWeight: FontWeight.w600,
            letterSpacing: 0.8,
          ),
          shape: const RoundedRectangleBorder(
            borderRadius: BorderRadius.all(DokRadius.none),
          ),
        ),
      ),
      textButtonTheme: TextButtonThemeData(
        style: TextButton.styleFrom(
          minimumSize: const Size(kMinTouchTarget, kMinTouchTarget),
          foregroundColor: DokColors.ink,
          textStyle: const TextStyle(
            fontSize: 13,
            fontWeight: FontWeight.w600,
            letterSpacing: 0.6,
          ),
        ),
      ),
      dividerTheme: const DividerThemeData(
        color: DokColors.border,
        thickness: 1,
        space: 1,
      ),
      cardTheme: CardThemeData(
        elevation: 0,
        color: DokColors.surface,
        margin: EdgeInsets.zero,
        shape: const RoundedRectangleBorder(
          borderRadius: BorderRadius.all(DokRadius.none),
        ),
      ),
      snackBarTheme: const SnackBarThemeData(
        behavior: SnackBarBehavior.floating,
        backgroundColor: DokColors.ink,
        contentTextStyle: TextStyle(color: DokColors.onButton, fontSize: 13),
      ),
      bottomSheetTheme: const BottomSheetThemeData(
        backgroundColor: DokColors.surface,
        surfaceTintColor: Colors.transparent,
        shape: RoundedRectangleBorder(
          borderRadius: BorderRadius.vertical(top: Radius.circular(12)),
        ),
      ),
      dialogTheme: const DialogThemeData(
        backgroundColor: DokColors.surface,
        surfaceTintColor: Colors.transparent,
      ),
    );
  }
}

// ---------------------------------------------------------------------------
// Backwards-compatible aliases.
//
// The admin console was written against the old `Gem*` token names. Renaming
// them without aliases would have forced a rewrite of every admin screen for
// no visual gain, so the old names stay as thin aliases onto the new
// palette. New code should use the `Dok*` names — these exist to keep the
// two clients compiling, not as a second way to spell the same thing.
// ---------------------------------------------------------------------------

class GemColors {
  const GemColors._();

  static const brand = DokColors.accent;
  static const brandDark = DokColors.accent;
  static const line = DokColors.line;
  static const ink = DokColors.ink;
  static const inkMuted = DokColors.inkSoft;
  static const surface = DokColors.surface;
  static const surfaceAlt = DokColors.surfaceAlt;
  static const border = DokColors.border;
  static const success = DokColors.success;
  static const warning = DokColors.warning;
  static const danger = DokColors.danger;
  static const info = DokColors.info;
}

class GemSpace {
  const GemSpace._();

  static const xs = DokSpace.xs;
  static const sm = DokSpace.sm;
  static const md = DokSpace.md;
  static const lg = DokSpace.lg;
  static const xl = DokSpace.xl;
}

class GemRadius {
  const GemRadius._();

  static const sm = DokRadius.sm;
  static const md = DokRadius.md;
  static const lg = DokRadius.lg;
}

class GemTheme {
  const GemTheme._();

  static ThemeData light({Color seed = DokColors.accent}) =>
      DokTheme.light(seed: seed);
}
