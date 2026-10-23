import { readFileSync, writeFileSync } from 'node:fs';

const f = 'D:/DOKIPON/packages/design_system/lib/src/widgets/storefront_app.dart';
let s = readFileSync(f, 'utf8');
const before = s;

// 1. The HeroBanner needs to know whether a category filter is actually applied.
s = s.replace(
  `class _HeroBanner extends StatelessWidget {
  const _HeroBanner({required this.onShopAll});

  /// "Show the whole collection" — clears the category filter the strip
  /// applies, so the button is the way back to everything after a shopper
  /// has narrowed the catalogue down.
  final VoidCallback onShopAll;`,
  `class _HeroBanner extends StatelessWidget {
  const _HeroBanner({
    required this.onShopAll,
    required this.hasCategoryFilter,
  });

  /// "Show the whole collection" — clears the category filter the strip
  /// applies, so the button is the way back to everything after a shopper
  /// has narrowed the catalogue down.
  final VoidCallback onShopAll;

  /// Whether a category is narrowing the catalogue down right now.
  ///
  /// When nothing is filtered, "show the whole collection" has nothing to
  /// clear. Offering the button anyway is a control that does nothing, so it
  /// is disabled instead of pretending to be actionable.
  final bool hasCategoryFilter;`,
);

// 2. Disable it when there is nothing to clear.
s = s.replace(
  `          OutlinedButton(
            onPressed: onShopAll,
            child: const Text('SHOP COLLECTION'),
          ),`,
  `          OutlinedButton(
            onPressed: hasCategoryFilter ? onShopAll : null,
            child: const Text('SHOP COLLECTION'),
          ),`,
);

// 3. Pass it at the call site.
s = s.replace(
  `SliverToBoxAdapter(child: _HeroBanner(onShopAll: _showAll)),`,
  `SliverToBoxAdapter(
              child: _HeroBanner(
                onShopAll: _showAll,
                hasCategoryFilter: selectedCategory != null,
              ),
            ),`,
);

if (s === before) {
  console.error('no replacement applied');
  process.exit(1);
}
writeFileSync(f, s, 'utf8');
console.log('storefront_app.dart rewritten');

// The audit must not count a deliberately disabled control as broken.
const a = 'D:/DOKIPON/tools/commerce-e2e/scripts/audit.mjs';
let js = readFileSync(a, 'utf8');
const jsBefore = js;
js = js.replace(
  `  if (clickError) {`,
  `  if (control.disabled) {
    // A control that is deliberately disabled is not a broken control: it says
    // "not right now", which is the truth. Clicking it must change nothing.
    skipped = 'disabled';
  } else if (clickError) {`,
);
if (js === jsBefore) {
  console.error('audit.mjs: no replacement applied');
  process.exit(1);
}
writeFileSync(a, js, 'utf8');
console.log('audit.mjs rewritten');
