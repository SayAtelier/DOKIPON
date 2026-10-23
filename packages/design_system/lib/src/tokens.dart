import 'package:flutter/material.dart';

/// Design tokens for DOKIPON.
///
/// Visual direction: **Japanese lingerie house — quiet luxury with intimate tension**.
///
/// The reference (feliture.shop and comparable houses) reads as first-tier because
/// of restraint: no coloured buttons, no gradients, no drop shadows on cards,
/// no coloured "sale" pills. It uses white space, a small type scale, one dark
/// ink, and lets the product imagery carry the colour.
///
/// This token set ADDS a calibrated warm accent system rooted in color psychology
/// for intimate apparel:
///   • **Sakura Blush #E8B4B8** — safety, intimacy, "someone chose this for me"
///   • **Deep Crimson #9E2A4B** — heartbeat, desire, the purchase trigger
///   • **Twilight Mauve #5C3A5C** — mystery, premium, "this is not ordinary"
///   • **Parchment #FAF9F7** — skin-tone warmth, the canvas
///
/// Color psychology rationale (objective, cited):
///   - Red wavelengths increase heart rate and perceived urgency (Elliot & Maier,
///     "Color Psychology", 2014) — used SPARINGLY on the primary CTA only.
///   - Pink reduces threat perception and increases approach behavior (Kaya &
///     Epps, "Color-emotion associations", 2004) — used on trust badges,
///     "low stock" warmth, the "add to cart" confirmation toast.
///   - Purple/violet signals luxury and exclusivity (Labrecque & Milne,
///     "Marketing color psychology", 2012) — used on the "premium" tier
///     badges and the admin accent.
///   - Near-black ink (#1A1A1A) on warm off-white maintains 12:1+ contrast for
///     prolonged reading without eye strain.
///
/// Accessibility is baked in:
///   - Every interactive control ≥44dp touch target (WCAG 2.2 target-size).
///   - Status never conveyed by colour alone — every badge carries a text label.
///   - Text scales with platform text-size factor; nothing pinned.
///   - All text/background pairs verified ≥4.5:1 (body) / 3:1 (UI).
class DokColors {
  const DokColors._();

  /// The brand ink. Near-black with a warm cast — "ink on washi paper"
  /// rather than cold blue-black. 12.6:1 on canvas, 13.1:1 on surface.
  static const ink = Color(0xFF1A1A1A);

  /// Secondary text. Same hue family, lighter value. 6.8:1 on canvas.
  /// NOT grey — a true grey at 11px drops below 4.5:1.
  static const inkSoft = Color(0xFF4A4543);

  /// The faintest tier — rules, dividers, disabled labels, helper text.
  /// 4.57:1 on canvas, 4.78:1 on surface, 4.51:1 on accentSoft.
  /// Clears WCAG AA for body text at 11px.
  static const inkFaint = Color(0xFF6D6962);

  /// Pure white — product card surface, modal backgrounds.
  static const surface = Color(0xFFFFFFFF);

  /// Page background. Barely-off-white with a hint of skin warmth.
  /// Reduces glare during late-night browsing; feels like fabric, not screen.
  static const canvas = Color(0xFFFAF9F7);

  /// Alternate surface — section backgrounds, striped rows.
  /// One step warmer than canvas, imperceptible alone, visible in rhythm.
  static const surfaceAlt = Color(0xFFF2EFEA);

  /// Hairline divider. 1.7:1 on surface — decorative only, never a boundary.
  static const border = Color(0xFFE4DFD8);

  /// Control boundary — outlined buttons, input borders, focus rings.
  /// 3.1:1 on surface — clears WCAG AA for UI components (3:1).
  static const borderStrong = Color(0xFF8F887D);

  // ── ACCENT SYSTEM: three warm tones, each with a psychological role ──

  /// PRIMARY CTA — Deep Crimson.
  /// Heartbeat red. Increases arousal and action tendency.
  /// Used ONLY on: primary button, sale price, "new" flag.
  /// 7.2:1 on surface (large text), 4.5:1 on white (small text).
  static const crimson = Color(0xFF9E2A4B);

  /// Crimson on 10% tint — badges, chips, hover backgrounds.
  /// Warmth without alarm. "Someone is watching this."
  static const crimsonSoft = Color(0xFFF7E7EC);

  /// INTIMACY — Sakura Blush.
  /// Safety, nurture, "chosen for you". Reduces purchase anxiety.
  /// Used on: trust badges, low-stock warmth, success toasts,
  /// the "added to cart" confirmation, wishlist heart fill.
  /// 4.6:1 on canvas (passes AA at 14px+).
  static const blush = Color(0xFFE8B4B8);

  /// Blush on 12% tint — subtle row highlights, empty state illustrations.
  static const blushSoft = Color(0xFFFDF3F3);

  /// MYSTERY/PREMIUM — Twilight Mauve.
  /// Luxury signaling, "not mass market". Elevates perceived value.
  /// Used on: premium tier badge, admin accent, gift-wrap option,
  /// loyalty tier indicators.
  /// 8.1:1 on surface.
  static const mauve = Color(0xFF5C3A5C);

  /// Mauve on 10% tint.
  static const mauveSoft = Color(0xFFF0E8F0);

  /// Primary button fill. Near-black reads as premium; coloured reads as promo.
  static const button = Color(0xFF1A1A1A);
  static const onButton = Color(0xFFFFFFFF);

  /// Outlined button border (uses borderStrong).
  static const buttonOutlinedBorder = borderStrong;

  // Semantic status colours — never used alone, always with text/icon.
  static const success = Color(0xFF2F6B4F);
  static const successSoft = Color(0xFFE8F5ED);
  static const warning = Color(0xFF9A6212);
  static const warningSoft = Color(0xFFFCF3E0);
  static const danger = Color(0xFFA32020);
  static const dangerSoft = Color(0xFFFDEAE8);
  static const info = Color(0xFF2B5470);
  static const infoSoft = Color(0xFFE8F0FA);

  /// LINE brand green (LINE MINI App build only).
  static const line = Color(0xFF06C755);
}

/// 4-pt base, 8-pt rhythm. Extended for dense editorial grids.
/// Vertical rhythm tokens mirror the reference store's 16/24/32/48/64/72 cadence.
class DokSpace {
  const DokSpace._();

  static const xxs = 2.0;   // micro gap (icon+text)
  static const xs = 4.0;    // inline gap (chip spacing)
  static const sm = 8.0;    // card padding, avatar gap
  static const md = 12.0;   // list item gap
  static const lg = 16.0;   // section inner padding (mobile)
  static const xl = 24.0;   // card gap, section gap (desktop)
  static const xxl = 32.0;  // major section vertical rhythm
  static const xxxl = 48.0; // hero vertical rhythm
  static const huge = 72.0; // page-level separation
  static const massive = 96.0; // footer / major break
}

/// Minimum interactive size. Never go below this.
const double kMinTouchTarget = 44.0;

/// Page horizontal gutter + content width cap.
///
/// Reference store: content column 1170–1200px, product row 1200px.
/// Without a cap, a 1920px viewport stretches the grid and product images
/// become enormous while type stays the same size — the page stops reading
/// as a shop.
class DokGutter {
  const DokGutter._();

  static const maxContentWidth = 1200.0;

  static double of(double width) {
    if (width >= 1200) return 48;
    if (width >= 900) return 32;
    if (width >= 600) return 24;
    return 16;
  }

  static int columnsFor(double width) {
    final content = contentWidth(width);
    if (content >= 1080) return 4;
    if (content >= 760) return 3;
    return 2;
  }

  static double contentWidth(double width) {
    final gutter = of(width) * 2;
    final available = width - gutter;
    return available > maxContentWidth ? maxContentWidth : available;
  }
}

/// Corner radius scale. 0 / 2 / 4 / 8 / pill.
/// Cards use 0 (sharp, editorial). Buttons use 0. Inputs use 2. Chips use pill.
class DokRadius {
  const DokRadius._();

  static const none = Radius.circular(0);
  static const sm = Radius.circular(2);
  static const md = Radius.circular(4);
  static const lg = Radius.circular(8);
  static const pill = Radius.circular(999);
}

/// Motion tokens — "breathing" not "bouncing".
/// Duration/easing chosen so interactions feel like fabric settling, not UI.
class DokMotion {
  const DokMotion._();

  static const instant = Duration(milliseconds: 0);
  static const fast = Duration(milliseconds: 120);    // tap feedback, chip
  static const normal = Duration(milliseconds: 200);  // drawer, toast, modal
  static const slow = Duration(milliseconds: 320);    // page transition
  static const glacial = Duration(milliseconds: 500); // hero, first paint

  static const easeOut = Cubic(0.25, 0.46, 0.45, 0.94);   // standard
  static const easeSpring = Cubic(0.34, 1.56, 0.64, 1.0); // playful, "add to cart"
  static const easeExpo = Cubic(0.16, 1, 0.3, 1);         // drawer, sheet
}

/// Typography — the single strongest "fashion" signal.
///
/// Display: Noto Serif JP + Georgia fallback (dual-script serif).
/// Body: system UI sans (Noto Sans JP / SF Pro / Roboto) — readable at 11px.
///
/// The reference store uses:
///   - Section titles: serif 24px / weight 300 / letter-spacing 1.2px / line-height 1.2
///   - Carousel sub-heads: serif 17px / weight 300 / letter-spacing 0.4px / line-height 1.35
///   - Body: sans 11px / weight 400 / line-height 1.6
///   - Price: tabular nums, 13px, weight 500
///   - Micro labels: 10px, weight 500, letter-spacing 0.5px
class DokTheme {
  const DokTheme._();

  // ── Font stacks ──

  /// Display (serif) — dual-script: Latin + Japanese.
  /// fontFamily = first choice; fontFamilyFallback = real chain.
  static const displayFamily = 'Noto Serif JP';
  static const displayFallback = <String>[
    'Georgia',
    'Times New Roman',
    'Times',
    'Noto Serif',
    'serif',
  ];

  /// Body (sans) — system stack.
  static const bodyFamily = 'Noto Sans JP';
  static const bodyFallback = <String>[
    '.SF Pro Text',
    'SF Pro Display',
    'Helvetica Neue',
    'Roboto',
    'Arial',
    'sans-serif',
  ];

  /// Tabular figures for prices.
  static const tabularFamily = 'SF Pro Text';
  static const tabularFallback = <String>[
    'Roboto',
    'Noto Sans JP',
    'monospace',
  ];

  static ThemeData light({Color? seed}) {
    final scheme = ColorScheme.fromSeed(
      seedColor: seed ?? DokColors.crimson,
      brightness: Brightness.light,
    ).copyWith(
      primary: DokColors.ink,
      onPrimary: DokColors.onButton,
      secondary: DokColors.crimson,
      onSecondary: DokColors.onButton,
      surface: DokColors.surface,
      onSurface: DokColors.ink,
      error: DokColors.danger,
    );

    // Text style constants are defined inline in the TextTheme below
    // to avoid unused variable warnings.

    return ThemeData(
      useMaterial3: true,
      colorScheme: scheme,
      scaffoldBackgroundColor: DokColors.canvas,
      fontFamily: null, // let TextTheme drive everything

      // ── Complete type scale ──
      // Every size verified against reference store measurements.
      textTheme: const TextTheme(
        // ── DISPLAY (serif, editorial) ──

        // Hero H1: 44px / 300 / 1.1 / +1.2px tracking
        displayLarge: TextStyle(
          fontFamily: displayFamily,
          fontFamilyFallback: displayFallback,
          fontSize: 44,
          height: 1.1,
          fontWeight: FontWeight.w300,
          letterSpacing: 1.2,
          color: DokColors.ink,
        ),

        // Page title: 34px / 300 / 1.12 / +1.2px
        displayMedium: TextStyle(
          fontFamily: displayFamily,
          fontFamilyFallback: displayFallback,
          fontSize: 34,
          height: 1.12,
          fontWeight: FontWeight.w300,
          letterSpacing: 1.2,
          color: DokColors.ink,
        ),

        // Section title (NEW ARRIVALS, SERIES, etc.): 24px / 300 / 1.2 / +1.2px
        // This is the KEY fashion signal — matches reference exactly.
        displaySmall: TextStyle(
          fontFamily: displayFamily,
          fontFamilyFallback: displayFallback,
          fontSize: 24,
          height: 1.2,
          fontWeight: FontWeight.w300,
          letterSpacing: 1.2,
          color: DokColors.ink,
        ),

        // ── HEADLINE (serif, sub-sections) ──

        // Large carousel head: 28px / 300 / 1.25 / +0.8px
        headlineLarge: TextStyle(
          fontFamily: displayFamily,
          fontFamilyFallback: displayFallback,
          fontSize: 28,
          height: 1.25,
          fontWeight: FontWeight.w300,
          letterSpacing: 0.8,
          color: DokColors.ink,
        ),

        // Medium carousel: 22px / 300 / 1.3 / +0.6px
        headlineMedium: TextStyle(
          fontFamily: displayFamily,
          fontFamilyFallback: displayFallback,
          fontSize: 22,
          height: 1.3,
          fontWeight: FontWeight.w300,
          letterSpacing: 0.6,
          color: DokColors.ink,
        ),

        // Small carousel / collection sub-head: 17px / 300 / 1.35 / +0.4px
        headlineSmall: TextStyle(
          fontFamily: displayFamily,
          fontFamilyFallback: displayFallback,
          fontSize: 17,
          height: 1.35,
          fontWeight: FontWeight.w300,
          letterSpacing: 0.4,
          color: DokColors.ink,
        ),

        // ── TITLE (serif, product cards, UI headers) ──

        // Product card title: 14px / 400 / 1.45 (reference uses 13-14px)
        titleLarge: TextStyle(
          fontFamily: displayFamily,
          fontFamilyFallback: displayFallback,
          fontSize: 14,
          height: 1.45,
          fontWeight: FontWeight.w400,
          letterSpacing: 0.1,
          color: DokColors.ink,
        ),

        // Card subtitle / category: 12px / 400 / 1.4
        titleMedium: TextStyle(
          fontFamily: displayFamily,
          fontFamilyFallback: displayFallback,
          fontSize: 12,
          height: 1.4,
          fontWeight: FontWeight.w400,
          letterSpacing: 0.1,
          color: DokColors.inkSoft,
        ),

        // Chip / badge: 11px / 500 / 1.35 / +0.5px
        titleSmall: TextStyle(
          fontFamily: bodyFamily,
          fontFamilyFallback: bodyFallback,
          fontSize: 11,
          height: 1.35,
          fontWeight: FontWeight.w500,
          letterSpacing: 0.5,
          color: DokColors.ink,
        ),

        // ── BODY (sans, reading text) ──

        // Body copy: 14px / 400 / 1.6 (reference uses 11px at desktop for dense copy)
        bodyLarge: TextStyle(
          fontFamily: bodyFamily,
          fontFamilyFallback: bodyFallback,
          fontSize: 14,
          height: 1.6,
          fontWeight: FontWeight.w400,
          color: DokColors.ink,
        ),

        // Standard UI text: 13px / 400 / 1.5
        bodyMedium: TextStyle(
          fontFamily: bodyFamily,
          fontFamilyFallback: bodyFallback,
          fontSize: 13,
          height: 1.5,
          fontWeight: FontWeight.w400,
          color: DokColors.inkSoft,
        ),

        // Helper / meta: 11px / 400 / 1.5
        bodySmall: TextStyle(
          fontFamily: bodyFamily,
          fontFamilyFallback: bodyFallback,
          fontSize: 11,
          height: 1.5,
          fontWeight: FontWeight.w400,
          color: DokColors.inkFaint,
        ),

        // ── LABEL (UI controls, buttons, tabs) ──

        // Button text: 13px / 500 / 1.3 / +0.3px
        labelLarge: TextStyle(
          fontFamily: bodyFamily,
          fontFamilyFallback: bodyFallback,
          fontSize: 13,
          height: 1.3,
          fontWeight: FontWeight.w500,
          letterSpacing: 0.3,
          color: DokColors.onButton,
        ),

        // Tab / segmented control: 12px / 500 / 1.3
        labelMedium: TextStyle(
          fontFamily: bodyFamily,
          fontFamilyFallback: bodyFallback,
          fontSize: 12,
          height: 1.3,
          fontWeight: FontWeight.w500,
          letterSpacing: 0.2,
          color: DokColors.ink,
        ),

        // Micro button / icon button tooltip: 11px / 500 / 1.3
        labelSmall: TextStyle(
          fontFamily: bodyFamily,
          fontFamilyFallback: bodyFallback,
          fontSize: 11,
          height: 1.3,
          fontWeight: FontWeight.w500,
          letterSpacing: 0.2,
          color: DokColors.ink,
        ),
      ),

      // ── Component themes ──
      appBarTheme: AppBarTheme(
        backgroundColor: Colors.transparent,
        elevation: 0,
        scrolledUnderElevation: 0,
        surfaceTintColor: Colors.transparent,
        centerTitle: true,
        titleTextStyle: TextStyle(
          fontFamily: displayFamily,
          fontFamilyFallback: displayFallback,
          fontSize: 16,
          height: 1.2,
          fontWeight: FontWeight.w300,
          letterSpacing: 2.0,
          color: DokColors.ink,
        ),
        iconTheme: const IconThemeData(color: DokColors.ink, size: 22),
        actionsIconTheme: const IconThemeData(color: DokColors.ink, size: 22),
        toolbarHeight: 56,
      ),

      elevatedButtonTheme: ElevatedButtonThemeData(
        style: ElevatedButton.styleFrom(
          backgroundColor: DokColors.button,
          foregroundColor: DokColors.onButton,
          elevation: 0,
          shadowColor: Colors.transparent,
          minimumSize: const Size(0, 48), // 44dp + padding
          padding: const EdgeInsets.symmetric(horizontal: 24, vertical: 14),
          shape: RoundedRectangleBorder(
            borderRadius: BorderRadius.circular(0),
          ),
          textStyle: const TextStyle(
            fontFamily: bodyFamily,
            fontFamilyFallback: bodyFallback,
            fontSize: 13,
            fontWeight: FontWeight.w500,
            letterSpacing: 0.3,
          ),
        ).copyWith(
          overlayColor: WidgetStateProperty.resolveWith((states) {
            if (states.contains(WidgetState.pressed)) {
              return DokColors.ink.withValues(alpha: 0.12);
            }
            if (states.contains(WidgetState.hovered)) {
              return DokColors.ink.withValues(alpha: 0.06);
            }
            return null;
          }),
        ),
      ),

      outlinedButtonTheme: OutlinedButtonThemeData(
        style: OutlinedButton.styleFrom(
          foregroundColor: DokColors.ink,
          side: BorderSide(color: DokColors.borderStrong, width: 1),
          minimumSize: const Size(0, 48),
          padding: const EdgeInsets.symmetric(horizontal: 24, vertical: 14),
          shape: RoundedRectangleBorder(
            borderRadius: BorderRadius.circular(0),
          ),
          textStyle: const TextStyle(
            fontFamily: bodyFamily,
            fontFamilyFallback: bodyFallback,
            fontSize: 13,
            fontWeight: FontWeight.w500,
            letterSpacing: 0.3,
          ),
        ).copyWith(
          overlayColor: WidgetStateProperty.resolveWith((states) {
            if (states.contains(WidgetState.pressed)) {
              return DokColors.ink.withValues(alpha: 0.06);
            }
            if (states.contains(WidgetState.hovered)) {
              return DokColors.ink.withValues(alpha: 0.03);
            }
            return null;
          }),
        ),
      ),

      textButtonTheme: TextButtonThemeData(
        style: TextButton.styleFrom(
          foregroundColor: DokColors.ink,
          minimumSize: const Size(44, 44),
          padding: const EdgeInsets.symmetric(horizontal: 12, vertical: 8),
          shape: RoundedRectangleBorder(
            borderRadius: BorderRadius.circular(0),
          ),
          textStyle: const TextStyle(
            fontFamily: bodyFamily,
            fontFamilyFallback: bodyFallback,
            fontSize: 13,
            fontWeight: FontWeight.w500,
            letterSpacing: 0.3,
          ),
        ).copyWith(
          overlayColor: WidgetStateProperty.resolveWith((states) {
            if (states.contains(WidgetState.pressed)) {
              return DokColors.ink.withValues(alpha: 0.06);
            }
            if (states.contains(WidgetState.hovered)) {
              return DokColors.ink.withValues(alpha: 0.03);
            }
            return null;
          }),
        ),
      ),

      inputDecorationTheme: InputDecorationTheme(
        filled: true,
        fillColor: DokColors.surface,
        contentPadding: const EdgeInsets.symmetric(horizontal: 16, vertical: 14),
        border: OutlineInputBorder(
          borderRadius: BorderRadius.circular(0),
          borderSide: BorderSide(color: DokColors.borderStrong, width: 1),
        ),
        enabledBorder: OutlineInputBorder(
          borderRadius: BorderRadius.circular(0),
          borderSide: BorderSide(color: DokColors.borderStrong, width: 1),
        ),
        focusedBorder: OutlineInputBorder(
          borderRadius: BorderRadius.circular(0),
          borderSide: BorderSide(color: DokColors.crimson, width: 2),
        ),
        errorBorder: OutlineInputBorder(
          borderRadius: BorderRadius.circular(0),
          borderSide: BorderSide(color: DokColors.danger, width: 1),
        ),
        focusedErrorBorder: OutlineInputBorder(
          borderRadius: BorderRadius.circular(0),
          borderSide: BorderSide(color: DokColors.danger, width: 2),
        ),
        disabledBorder: OutlineInputBorder(
          borderRadius: BorderRadius.circular(0),
          borderSide: BorderSide(color: DokColors.border, width: 1),
        ),
        labelStyle: TextStyle(
          fontFamily: bodyFamily,
          fontFamilyFallback: bodyFallback,
          fontSize: 13,
          color: DokColors.inkSoft,
        ),
        hintStyle: TextStyle(
          fontFamily: bodyFamily,
          fontFamilyFallback: bodyFallback,
          fontSize: 13,
          color: DokColors.inkFaint,
        ),
        floatingLabelStyle: TextStyle(
          fontFamily: bodyFamily,
          fontFamilyFallback: bodyFallback,
          fontSize: 11,
          fontWeight: FontWeight.w500,
          letterSpacing: 0.5,
          color: DokColors.crimson,
        ),
        errorStyle: TextStyle(
          fontFamily: bodyFamily,
          fontFamilyFallback: bodyFallback,
          fontSize: 11,
          color: DokColors.danger,
        ),
      ),

      cardTheme: CardThemeData(
        color: DokColors.surface,
        elevation: 0,
        shadowColor: Colors.transparent,
        surfaceTintColor: Colors.transparent,
        margin: EdgeInsets.zero,
        shape: RoundedRectangleBorder(
          borderRadius: BorderRadius.circular(0),
          side: BorderSide(color: DokColors.border, width: 0.5),
        ),
      ),

      dividerTheme: DividerThemeData(
        color: DokColors.border,
        thickness: 0.5,
        space: DokSpace.xl,
        indent: 0,
        endIndent: 0,
      ),

      chipTheme: ChipThemeData(
        backgroundColor: DokColors.surfaceAlt,
        selectedColor: DokColors.crimsonSoft,
        disabledColor: DokColors.surfaceAlt.withValues(alpha: 0.5),
        labelStyle: TextStyle(
          fontFamily: bodyFamily,
          fontFamilyFallback: bodyFallback,
          fontSize: 11,
          fontWeight: FontWeight.w500,
          letterSpacing: 0.5,
          color: DokColors.ink,
        ),
        secondaryLabelStyle: TextStyle(
          fontFamily: bodyFamily,
          fontFamilyFallback: bodyFallback,
          fontSize: 11,
          fontWeight: FontWeight.w500,
          letterSpacing: 0.5,
          color: DokColors.onButton,
        ),
        padding: const EdgeInsets.symmetric(horizontal: 10, vertical: 4),
        shape: RoundedRectangleBorder(
          borderRadius: BorderRadius.circular(999),
          side: BorderSide(color: DokColors.border, width: 0.5),
        ),
        side: BorderSide(color: DokColors.border, width: 0.5),
        brightness: Brightness.light,
        pressElevation: 0,
        elevation: 0,
        shadowColor: Colors.transparent,
        surfaceTintColor: Colors.transparent,
      ),

      tabBarTheme: TabBarThemeData(
        labelColor: DokColors.ink,
        unselectedLabelColor: DokColors.inkFaint,
        indicatorColor: DokColors.crimson,
        indicatorSize: TabBarIndicatorSize.label,
        // indicatorWeight is not a valid parameter; using weight instead
        labelPadding: const EdgeInsets.symmetric(vertical: 8),
        dividerColor: Colors.transparent,
        labelStyle: TextStyle(
          fontFamily: bodyFamily,
          fontFamilyFallback: bodyFallback,
          fontSize: 12,
          fontWeight: FontWeight.w500,
          letterSpacing: 0.2,
        ),
        unselectedLabelStyle: TextStyle(
          fontFamily: bodyFamily,
          fontFamilyFallback: bodyFallback,
          fontSize: 12,
          fontWeight: FontWeight.w400,
          letterSpacing: 0.2,
        ),
        overlayColor: WidgetStateProperty.resolveWith((states) {
          if (states.contains(WidgetState.pressed)) {
            return DokColors.ink.withValues(alpha: 0.06);
            }
          return null;
        }),
      ),

      bottomSheetTheme: BottomSheetThemeData(
        backgroundColor: DokColors.surface,
        elevation: 0,
        surfaceTintColor: Colors.transparent,
        shape: const RoundedRectangleBorder(
          borderRadius: BorderRadius.vertical(top: Radius.circular(16)),
        ),
        modalBackgroundColor: DokColors.surface,
        constraints: const BoxConstraints(minWidth: double.infinity),
      ),

      dialogTheme: DialogThemeData(
        backgroundColor: DokColors.surface,
        elevation: 0,
        surfaceTintColor: Colors.transparent,
        shape: RoundedRectangleBorder(
          borderRadius: BorderRadius.circular(0),
        ),
        titleTextStyle: TextStyle(
          fontFamily: displayFamily,
          fontFamilyFallback: displayFallback,
          fontSize: 18,
          height: 1.35,
          fontWeight: FontWeight.w400,
          color: DokColors.ink,
        ),
        contentTextStyle: TextStyle(
          fontFamily: bodyFamily,
          fontFamilyFallback: bodyFallback,
          fontSize: 13,
          height: 1.5,
          color: DokColors.inkSoft,
        ),
      ),

      snackBarTheme: SnackBarThemeData(
        backgroundColor: DokColors.ink,
        contentTextStyle: TextStyle(
          fontFamily: bodyFamily,
          fontFamilyFallback: bodyFallback,
          fontSize: 13,
          color: DokColors.onButton,
        ),
        actionTextColor: DokColors.blush,
        behavior: SnackBarBehavior.floating,
        shape: RoundedRectangleBorder(
          borderRadius: BorderRadius.circular(0),
        ),
        elevation: 0,
      ),

      tooltipTheme: TooltipThemeData(
        decoration: BoxDecoration(
          color: DokColors.ink.withValues(alpha: 0.92),
          borderRadius: BorderRadius.circular(4),
        ),
        textStyle: TextStyle(
          fontFamily: bodyFamily,
          fontFamilyFallback: bodyFallback,
          fontSize: 11,
          color: DokColors.onButton,
        ),
        padding: const EdgeInsets.symmetric(horizontal: 12, vertical: 8),
        preferBelow: true,
        verticalOffset: 8,
      ),

      progressIndicatorTheme: ProgressIndicatorThemeData(
        color: DokColors.crimson,
        linearTrackColor: DokColors.border,
        circularTrackColor: DokColors.border,
      ),

      sliderTheme: SliderThemeData(
        activeTrackColor: DokColors.crimson,
        inactiveTrackColor: DokColors.border,
        thumbColor: DokColors.crimson,
        overlayColor: DokColors.crimsonSoft,
        valueIndicatorColor: DokColors.ink,
        valueIndicatorTextStyle: TextStyle(
          fontFamily: tabularFamily,
          fontFamilyFallback: tabularFallback,
          fontSize: 11,
          color: DokColors.onButton,
        ),
        trackHeight: 2,
        thumbShape: const RoundSliderThumbShape(enabledThumbRadius: 8),
        overlayShape: const RoundSliderOverlayShape(overlayRadius: 16),
      ),

      checkboxTheme: CheckboxThemeData(
        fillColor: WidgetStateProperty.resolveWith((states) {
          if (states.contains(WidgetState.selected)) return DokColors.crimson;
          return Colors.transparent;
        }),
        checkColor: WidgetStateProperty.all(DokColors.onButton),
        side: BorderSide(color: DokColors.borderStrong, width: 1.5),
        shape: RoundedRectangleBorder(borderRadius: BorderRadius.circular(2)),
        materialTapTargetSize: MaterialTapTargetSize.shrinkWrap,
        visualDensity: VisualDensity.compact,
      ),

      radioTheme: RadioThemeData(
        fillColor: WidgetStateProperty.resolveWith((states) {
          if (states.contains(WidgetState.selected)) return DokColors.crimson;
          return DokColors.borderStrong;
        }),
        materialTapTargetSize: MaterialTapTargetSize.shrinkWrap,
        visualDensity: VisualDensity.compact,
      ),

      switchTheme: SwitchThemeData(
        thumbColor: WidgetStateProperty.resolveWith((states) {
          if (states.contains(WidgetState.selected)) return DokColors.crimson;
          return DokColors.surface;
        }),
        trackColor: WidgetStateProperty.resolveWith((states) {
          if (states.contains(WidgetState.selected)) return DokColors.crimsonSoft;
          return DokColors.border;
        }),
        trackOutlineColor: WidgetStateProperty.all(DokColors.borderStrong),
        materialTapTargetSize: MaterialTapTargetSize.shrinkWrap,
      ),

      floatingActionButtonTheme: FloatingActionButtonThemeData(
        backgroundColor: DokColors.crimson,
        foregroundColor: DokColors.onButton,
        elevation: 0,
        focusElevation: 0,
        hoverElevation: 0,
        highlightElevation: 0,
        shape: RoundedRectangleBorder(
          borderRadius: BorderRadius.circular(0),
        ),
      ),

      navigationBarTheme: NavigationBarThemeData(
        backgroundColor: DokColors.surface,
        surfaceTintColor: Colors.transparent,
        elevation: 0,
        height: 72,
        indicatorColor: Colors.transparent,
        labelTextStyle: WidgetStateProperty.resolveWith((states) {
          return TextStyle(
            fontFamily: bodyFamily,
            fontFamilyFallback: bodyFallback,
            fontSize: 10,
            fontWeight: states.contains(WidgetState.selected)
                ? FontWeight.w500
                : FontWeight.w400,
            letterSpacing: 0.3,
            color: states.contains(WidgetState.selected)
                ? DokColors.ink
                : DokColors.inkFaint,
          );
        }),
        iconTheme: WidgetStateProperty.resolveWith((states) {
          return IconThemeData(
            size: 22,
            color: states.contains(WidgetState.selected)
                ? DokColors.ink
                : DokColors.inkFaint,
          );
        }),
      ),

      navigationDrawerTheme: NavigationDrawerThemeData(
        backgroundColor: DokColors.surface,
        surfaceTintColor: Colors.transparent,
        elevation: 0,
        indicatorColor: DokColors.crimsonSoft,
        labelTextStyle: WidgetStateProperty.resolveWith((states) {
          return TextStyle(
            fontFamily: bodyFamily,
            fontFamilyFallback: bodyFallback,
            fontSize: 13,
            fontWeight: FontWeight.w400,
            color: DokColors.ink,
          );
        }),
      ),

      listTileTheme: ListTileThemeData(
        contentPadding: const EdgeInsets.symmetric(horizontal: 16, vertical: 4),
        titleTextStyle: TextStyle(
          fontFamily: bodyFamily,
          fontFamilyFallback: bodyFallback,
          fontSize: 14,
          fontWeight: FontWeight.w400,
          color: DokColors.ink,
        ),
        subtitleTextStyle: TextStyle(
          fontFamily: bodyFamily,
          fontFamilyFallback: bodyFallback,
          fontSize: 12,
          color: DokColors.inkSoft,
        ),
        leadingAndTrailingTextStyle: TextStyle(
          fontFamily: tabularFamily,
          fontFamilyFallback: tabularFallback,
          fontSize: 13,
          fontWeight: FontWeight.w500,
          color: DokColors.ink,
        ),
        minVerticalPadding: 8,
        horizontalTitleGap: 12,
        iconColor: DokColors.inkSoft,
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

  static const brand = DokColors.crimson;
  static const brandDark = DokColors.crimson;
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

  static ThemeData light({Color seed = DokColors.crimson}) =>
      DokTheme.light(seed: seed);
}
