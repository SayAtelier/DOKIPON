import 'package:api_client/api_client.dart';
import 'package:design_system/design_system.dart';
import 'package:domain_models/domain_models.dart';
import 'package:flutter/material.dart';
import 'package:storefront_state/storefront_state.dart';

/// The whole storefront UI, shared by all three clients.
///
/// The only thing that differs between App / Web / LINE is the seed
/// [Environment] and the theme seed colour. Everything below — catalogue,
/// product detail, checkout status — is one implementation, which is the
/// concrete form of `FR-CORE-05`.
class StorefrontApp extends StatefulWidget {
  const StorefrontApp({
    super.key,
    required this.environment,
    required this.api,
  });

  final Environment environment;
  final CommerceApiClient api;

  @override
  State<StorefrontApp> createState() => _StorefrontAppState();
}

class _StorefrontAppState extends State<StorefrontApp> {
  late final SessionController session;
  late final CatalogController catalog;
  late final CartController cart;
  late final CheckoutController checkout;
  late StorefrontConfig config;
  bool _booting = true;

  @override
  void initState() {
    super.initState();
    session = SessionController(api: widget.api);
    catalog = CatalogController(widget.api);
    cart = CartController(widget.api);
    checkout = CheckoutController(widget.api);
    config = StorefrontConfig.fallback(widget.environment.channel);
    _boot();
  }

  Future<void> _boot() async {
    await session.boot();
    try {
      config = await widget.api.fetchStorefront();
      checkout.configure(config);
    } on ApiProblem {
      // Keep the fallback config; the catalogue still loads.
    }
    await catalog.load();
    // The cart comes back with the session.
    //
    // The cart is identified per session, so a shopper who comes back finds
    // the basket they left. Without this the cart was only ever fetched when
    // something was added to it, and a reload showed an empty basket even
    // though the server still had it.
    final current = session.session;
    if (current != null) {
      await cart.load(current.sessionId);
    }
    if (mounted) {
      setState(() => _booting = false);
    }
  }

  @override
  void dispose() {
    session.dispose();
    catalog.dispose();
    cart.dispose();
    checkout.dispose();
    super.dispose();
  }

  @override
  Widget build(BuildContext context) {
    final seed = widget.environment.channel == StoreChannel.line
        ? DokColors.line
        : DokColors.crimson; // was accent
    return MaterialApp(
      title: 'DOKIPON',
      debugShowCheckedModeBanner: false,
      theme: DokTheme.light(seed: seed),
      home: CatalogScreen(
        session: session,
        catalog: catalog,
        cart: cart,
        checkout: checkout,
        booting: _booting,
      ),
    );
  }
}

/// The storefront screen, shared by every channel.
class CatalogScreen extends StatefulWidget {
  const CatalogScreen({
    super.key,
    required this.session,
    required this.catalog,
    required this.cart,
    required this.checkout,
    this.booting = false,
  });

  final SessionController session;
  final CatalogController catalog;
  final CartController cart;
  final CheckoutController checkout;
  final bool booting;

  @override
  State<CatalogScreen> createState() => _CatalogScreenState();
}

class _CatalogScreenState extends State<CatalogScreen> {
  /// Whether the cart drawer is open.
  bool _cartOpen = false;

  /// The handle on the `Scaffold` that owns the cart drawer.
  ///
  /// The drawer cannot be opened through `Scaffold.of(context)`: this State's
  /// context sits **above** the `Scaffold` — the `Scaffold` is built inside
  /// `build()` — so `Scaffold.of` finds no `Scaffold` ancestor and throws
  /// "Scaffold.of() called with a context that does not contain a Scaffold."
  /// A key reaches the `ScaffoldState` directly, whatever the tree does.
  final _scaffoldKey = GlobalKey<ScaffoldState>();

  @override
  void initState() {
    super.initState();
    widget.session.addListener(_onChange);
    widget.catalog.addListener(_onChange);
    widget.cart.addListener(_onChange);
    widget.checkout.addListener(_onChange);
  }

  @override
  void dispose() {
    widget.session.removeListener(_onChange);
    widget.catalog.removeListener(_onChange);
    widget.cart.removeListener(_onChange);
    widget.checkout.removeListener(_onChange);
    super.dispose();
  }

  void _onChange() {
    if (mounted) setState(() {});
  }

  /// Add a product to the cart.
  ///
  /// The catalogue only carries a summary, so the server's cart is the one
  /// that knows the line's snapshot price. A first add has no cart to mutate
  /// yet, so the client creates the cart id and loads it.
  Future<void> _addToCart(ProductSummary product, {bool fromSheet = false}) async {
    final cart = widget.cart;
    if (cart.cartId == null) {
      // The cart is identified per session. A guest session id is the
      // natural key: the server scopes the cart by owner, and the guest
      // session is what the shopper has at this point.
      final session = widget.session.session;
      if (session == null) return;
      await cart.load(session.sessionId);
    }
    if (cart.cartId == null) return;
    final added = await cart.add(product.sku);
    if (!mounted) return;
    if (added) {
      if (fromSheet) {
        // The sheet's button said "add to cart" and did not add. Close it
        // only once the item is really in the cart.
        Navigator.of(context).maybePop();
      }
      if (!_cartOpen) {
        ScaffoldMessenger.of(context).showSnackBar(
          SnackBar(
            content: Text('カートに追加しました / Added to cart: ${product.title}'),
            duration: const Duration(seconds: 2),
          ),
        );
      }
    } else if (fromSheet) {
      // The sheet stays open so the shopper can see the error and retry.
    }
  }

  @override
  Widget build(BuildContext context) {
    final catalog = widget.catalog;
    final session = widget.session;
    // The applied category filter, if any. The controller owns it — the
    // nav only reflects it and asks for a change.
    final selectedCategory = catalog.categoryId;

    // The products the shopper is looking at: the loaded page, narrowed by
    // the search term.
    final visible = _visibleItems(catalog.items);

    return Scaffold(
      key: _scaffoldKey,
      backgroundColor: DokColors.canvas,
      body: RefreshIndicator(
        onRefresh: () => catalog.load(refresh: true),
        color: DokColors.ink,
        child: CustomScrollView(
          physics: const AlwaysScrollableScrollPhysics(),
          slivers: [
            SliverToBoxAdapter(
              child: _BrandHeader(
                cart: widget.cart,
                onCartTap: _openCart,
                onSearchTap: _openSearch,
                onAccountTap: _openAccount,
              ),
            ),
            SliverToBoxAdapter(
              child: _CategoryNav(
                selectedCategory: catalog.categoryId,
                onSelect: (id) => catalog.selectCategory(id),
              ),
            ),
            if (session.notice != null)
              SliverToBoxAdapter(
                child: _Notice(
                  message: session.notice!,
                  actionLabel: 'OK',
                  onAction: session.clearNotice,
                ),
              ),
            if (session.isGuest && !session.isBooting)
              const SliverToBoxAdapter(
                child: _Notice(
                  message: 'ゲスト表示です。購入にはログインが必要です / '
                      'Browsing as a guest — sign in to check out.',
                ),
              ),
            SliverToBoxAdapter(
              child: _HeroBanner(
                onShopAll: _showAll,
                hasCategoryFilter: selectedCategory != null,
              ),
            ),
            SliverToBoxAdapter(
              child: _SectionRule(
                selectedCategory == null
                    ? 'NEW ARRIVALS'
                    : _CategoryNav.labelOf(selectedCategory),
              ),
            ),
            if (widget.booting || (catalog.isLoading && catalog.items.isEmpty))
              const SliverToBoxAdapter(
                child: Padding(
                  padding: EdgeInsets.all(DokSpace.huge),
                  child: Center(child: CircularProgressIndicator(color: DokColors.ink)),
                ),
              )
            else if (catalog.error != null && catalog.items.isEmpty)
              SliverToBoxAdapter(
                child: Padding(
                  padding: const EdgeInsets.all(DokSpace.lg),
                  child: StatusBanner(
                    message: catalog.error!.detail.isEmpty
                        ? '商品を読み込めませんでした / Could not load products.'
                        : catalog.error!.detail,
                    tone: BannerTone.danger,
                    actionLabel: '再試行 / Retry',
                    onAction: () => catalog.load(refresh: true),
                  ),
                ),
              )
            else if (catalog.isEmpty || visible.isEmpty)
              SliverToBoxAdapter(
                child: Padding(
                  padding: const EdgeInsets.all(DokSpace.huge),
                  child: Center(
                    child: Text(
                      // Three different emptinesses, three different
                      // messages: the catalogue is empty, the chosen category
                      // is empty, or the search matched nothing.
                      !catalog.isEmpty
                          ? '条件に一致する商品がありません / No products match'
                          : selectedCategory == null
                              ? '商品がありません / No products available'
                              : '「${_CategoryNav.labelOf(selectedCategory)}」の商品が'
                                  'ありません / No products in '
                                  '${_CategoryNav.labelOf(selectedCategory)}',
                      textAlign: TextAlign.center,
                    ),
                  ),
                ),
              )
            else
              SliverPadding(
                padding: EdgeInsets.symmetric(
                  horizontal: _gutterOf(context),
                  vertical: DokSpace.lg,
                ),
                sliver: SliverLayoutBuilder(
                  builder: (context, constraints) {
                    final width = constraints.crossAxisExtent;
                    final columns = DokGutter.columnsFor(width);
                    // The card's height comes from its content — a square
                    // photo, then the title, price and stock line. A fixed
                    // aspect ratio on the whole tile made every card twice
                    // as tall as it was wide, which at 1024px produced
                    // 472x908px tiles with a third of the viewport given over
                    // to one product's image.
                    final cardWidth =
                        (width - DokSpace.lg * (columns - 1)) / columns;
                    final imageHeight = cardWidth; // 1:1, as the reference store
                    final textHeight = 96.0;
                    final extent = imageHeight + textHeight;
                    return SliverGrid(
                      gridDelegate: SliverGridDelegateWithFixedCrossAxisCount(
                        crossAxisCount: columns,
                        mainAxisSpacing: DokSpace.xxl,
                        crossAxisSpacing: DokSpace.lg,
                        mainAxisExtent: extent,
                      ),
                      delegate: SliverChildBuilderDelegate(
                        (context, index) {
                          final product = visible[index];
                          return ProductCard(
                            product: product,
                            onOpen: () => _showDetail(product),
                            onAdd: () => _addToCart(product),
                          );
                        },
                        childCount: visible.length,
                      ),
                    );
                  },
                ),
              ),
            if (catalog.hasMore)
              SliverToBoxAdapter(
                child: Padding(
                  padding: const EdgeInsets.all(DokSpace.lg),
                  child: Center(
                    child: OutlinedButton(
                      onPressed: catalog.isLoadingMore ? null : catalog.loadMore,
                      child: catalog.isLoadingMore
                          ? const SizedBox(
                              width: 18,
                              height: 18,
                              child: CircularProgressIndicator(strokeWidth: 2),
                            )
                          : const Text('もっと見る / Load more'),
                    ),
                  ),
                ),
              ),
            const SliverToBoxAdapter(child: _Footer()),
            const SliverToBoxAdapter(child: SizedBox(height: DokSpace.xxl)),
          ],
        ),
      ),
      bottomNavigationBar: CheckoutStatusBar(checkout: widget.checkout),
      drawer: CartDrawer(
        cart: widget.cart,
        isMutating: widget.cart.isMutating,
        error: widget.cart.error,
        canCheckout: widget.session.canCheckout,
        onClose: _closeCart,
        onQuantityChanged: (sku, quantity) =>
            widget.cart.setQuantity(sku: sku, quantity: quantity),
        onRemove: (sku) => widget.cart.remove(sku),
        onCheckout: _checkout,
      ),
      onDrawerChanged: (open) {
        if (open != _cartOpen) setState(() => _cartOpen = open);
      },
    );
  }

  void _openCart() {
    _scaffoldKey.currentState?.openDrawer();
  }

  void _closeCart() {
    Navigator.of(context).maybePop();
  }

  /// The search term.
  ///
  /// The catalogue route has no text-search parameter
  /// (`contracts/openapi.yaml` -> `GET /v1/products` takes `cursor`, `limit`,
  /// `locale` and `categoryId`), so this narrows the products that are
  /// **on screen** rather than the whole catalogue — and the field says so,
  /// in the same words, so nobody is promised a search that is not there.
  String _query = '';

  /// The products the shopper is looking at.
  List<ProductSummary> _visibleItems(List<ProductSummary> items) {
    final q = _query.trim().toLowerCase();
    if (q.isEmpty) return items;
    return items
        .where((p) => p.title.toLowerCase().contains(q))
        .toList(growable: false);
  }

  /// Ask for the whole collection again — the way back to everything after
  /// the category strip has narrowed the catalogue down.
  Future<void> _showAll() async {
    await widget.catalog.selectCategory(null);
  }

  /// Search the products on screen.
  Future<void> _openSearch() async {
    final controller = TextEditingController(text: _query);
    final result = await showDialog<String>(
      context: context,
      builder: (context) => AlertDialog(
        title: const Text('商品を探す / Search products'),
        content: TextField(
          controller: controller,
          autofocus: true,
          decoration: const InputDecoration(
            labelText: '表示中の商品を絞り込む / Filter the products on screen',
          ),
        ),
        actions: [
          TextButton(
            onPressed: () => Navigator.of(context).pop(),
            child: const Text('キャンセル / Cancel'),
          ),
          TextButton(
            onPressed: () => Navigator.of(context).pop(controller.text.trim()),
            child: const Text('適用 / Apply'),
          ),
        ],
      ),
    );
    controller.dispose();
    if (result == null) return;
    setState(() => _query = result);
  }

  /// Who the shopper is, and what they are allowed to do.
  ///
  /// A guest can browse but not check out; a LINE user can do both. The
  /// degraded notice (LIFF-002..006) is shown here too, because the session
  /// controller is the only place that knows about it.
  Future<void> _openAccount() async {
    final session = widget.session;
    await showModalBottomSheet<void>(
      context: context,
      showDragHandle: true,
      builder: (context) {
        final theme = Theme.of(context);
        return Padding(
          padding: const EdgeInsets.fromLTRB(24, 0, 24, 32),
          child: Column(
            mainAxisSize: MainAxisSize.min,
            crossAxisAlignment: CrossAxisAlignment.start,
            children: [
              // The sheet has to be dismissible by a control, not only by
              // tapping the barrier. A shopper who cannot tap the
              // background — with a keyboard, or with a screen reader — had
              // no way out of this sheet. The cart drawer's close button is
              // the pattern, and the same label, so the two sheets are
              // closed the same way.
              Row(
                children: [
                  Expanded(
                    child: Text('アカウント / Account', style: theme.textTheme.titleLarge),
                  ),
                  IconButton(
                    onPressed: () => Navigator.of(context).pop(),
                    icon: const Icon(Icons.close, size: 20),
                    color: DokColors.ink,
                    tooltip: '閉じる / Close',
                  ),
                ],
              ),
              const SizedBox(height: 12),
              Text(
                session.isGuest
                    ? 'セッション / Session: ゲスト / Guest'
                    : 'セッション / Session: LINE ユーザー / LINE user',
                style: theme.textTheme.bodyMedium,
              ),
              const SizedBox(height: 6),
              Text(
                session.canCheckout
                    ? '購入できます / You can check out.'
                    : 'LINE の中ではログインが必要です / '
                        'Inside LINE, sign in to check out.',
                style: theme.textTheme.bodyMedium,
              ),
              if (session.notice != null) ...[
                const SizedBox(height: 10),
                Text(session.notice!, style: theme.textTheme.bodySmall),
              ],
            ],
          ),
        );
      },
    );
  }

  /// Submit the checkout.
  ///
  /// The payload's line prices are the cart's snapshot prices; the server
  /// discards them and re-prices from the catalogue. The idempotency key is
  /// derived from the cart's identity, so a transport-level retry replays the
  /// same checkout instead of charging twice.
  Future<void> _checkout() async {
    final cart = widget.cart;
    if (cart.isEmpty) return;
    // The one state in which this session may not order: the degraded state
    // inside the LINE app. The button is already dead there; this is the
    // same rule on the path itself, so a caller that is not the button
    // cannot slip past it.
    if (!widget.session.canCheckout) return;
    final lines = cart.lines;
    final payload = CommerceApiClient.buildCheckoutPayload(
      lines: lines,
      paymentMethodRef: 'tok_test',
    );
    await widget.checkout.submit(
      operation: 'checkout.cart-${cart.cartId}',
      payload: payload,
    );
    if (mounted) {
      _scaffoldKey.currentState?.closeDrawer();
    }
  }

  static double _gutterOf(BuildContext context) =>
      DokGutter.of(MediaQuery.sizeOf(context).width);

  void _showDetail(ProductSummary product) {
    showModalBottomSheet<void>(
      context: context,
      showDragHandle: true,
      isScrollControlled: true,
      builder: (_) => _ProductDetailSheet(
        product: product,
        isMutating: widget.cart.isMutating,
        error: widget.cart.error,
        onAdd: () => _addToCart(product, fromSheet: true),
      ),
    );
  }
}


/// Centres the page content and caps its width.
///
/// The reference store measures 1170–1200px for its content column. Without
/// a cap, a 1920px screen spreads the catalogue edge to edge and the
/// product images become enormous while the type stays the same size.
class _ContentWidth extends StatelessWidget {
  const _ContentWidth({required this.child});

  final Widget child;

  @override
  Widget build(BuildContext context) {
    final width = MediaQuery.sizeOf(context).width;
    final gutter = DokGutter.of(width);
    return Center(
      child: ConstrainedBox(
        constraints: const BoxConstraints(maxWidth: DokGutter.maxContentWidth),
        child: Padding(
          padding: EdgeInsets.symmetric(horizontal: gutter),
          child: child,
        ),
      ),
    );
  }
}

/// The brand bar: wordmark centred, utilities at the edges, rules top and
/// bottom. The reference storefront does exactly this — the brand is the
/// loudest thing on the page and the utility icons stay quiet.
class _BrandHeader extends StatelessWidget {
  const _BrandHeader({
    required this.cart,
    required this.onCartTap,
    required this.onSearchTap,
    required this.onAccountTap,
  });

  final CartController cart;
  final VoidCallback onCartTap;
  final VoidCallback onSearchTap;
  final VoidCallback onAccountTap;

  @override
  Widget build(BuildContext context) {
    final width = MediaQuery.sizeOf(context).width;
    final compact = width < 600;

    return Container(
      color: DokColors.surface,
      padding: const EdgeInsets.symmetric(vertical: DokSpace.md),
      child: _ContentWidth(
        child: Column(
        children: [
          Row(
            children: [
              if (!compact)
                _UtilityIcon(
                  icon: Icons.search_outlined,
                  label: '検索 / Search',
                  onTap: onSearchTap,
                ),
              const Spacer(),
              const Text(
                'DOKIPON',
                style: TextStyle(
                  fontFamily: DokTheme.displayFamily,
                  fontSize: 26,
                  letterSpacing: 6,
                  height: 1,
                  color: DokColors.ink,
                ),
              ),
              const Spacer(),
              if (!compact) ...[
                _CartButton(count: cart.itemCount, onTap: onCartTap),
                _UtilityIcon(
                  icon: Icons.person_outline,
                  label: 'アカウント / Account',
                  onTap: onAccountTap,
                ),
              ],
            ],
          ),
          if (compact) ...[
            const SizedBox(height: DokSpace.sm),
            Row(
              mainAxisAlignment: MainAxisAlignment.spaceBetween,
              children: [
                _UtilityIcon(
                  icon: Icons.search_outlined,
                  label: '検索',
                  onTap: onSearchTap,
                ),
                _CartButton(count: cart.itemCount, onTap: onCartTap),
                _UtilityIcon(
                  icon: Icons.person_outline,
                  label: 'アカウント',
                  onTap: onAccountTap,
                ),
              ],
            ),
          ],
        ],
        ),
      ),
    );
  }
}

/// The cart icon, with a count badge when the cart is not empty.
///
/// A shopper needs to know at a glance whether anything is in the cart; a
/// bare bag icon that looks identical whether it holds three items or none
/// costs a click every time.
class _CartButton extends StatelessWidget {
  const _CartButton({required this.count, required this.onTap});

  final int count;
  final VoidCallback onTap;

  @override
  Widget build(BuildContext context) {
    return Semantics(
      button: true,
      label: count == 0
          ? 'カート / Cart, empty'
          : 'カート / Cart, $count items',
      excludeSemantics: true,
      child: InkWell(
        onTap: onTap,
        child: SizedBox(
          width: kMinTouchTarget,
          height: kMinTouchTarget,
          child: Stack(
            alignment: Alignment.center,
            children: [
              const Icon(Icons.shopping_bag_outlined, size: 20, color: DokColors.ink),
              if (count > 0)
                Positioned(
                  right: 4,
                  top: 6,
                  child: Container(
                    padding: const EdgeInsets.symmetric(horizontal: 4, vertical: 1),
                    constraints: const BoxConstraints(minWidth: 16),
                    color: DokColors.crimson, // was accent
                    child: Text(
                      '$count',
                      textAlign: TextAlign.center,
                      style: const TextStyle(
                        color: DokColors.onButton,
                        fontSize: 9,
                        height: 1.2,
                        fontWeight: FontWeight.w700,
                      ),
                    ),
                  ),
                ),
            ],
          ),
        ),
      ),
    );
  }
}

class _UtilityIcon extends StatelessWidget {
  const _UtilityIcon({
    required this.icon,
    required this.label,
    required this.onTap,
  });

  final IconData icon;
  final String label;

  /// Every utility icon does something.
  ///
  /// The action is part of the constructor on purpose: a control that is
  /// announced as a button and does nothing is worse than no control at all,
  /// and an empty `onTap: () {}` made that possible twice. A new utility icon
  /// cannot be added without deciding what it does.
  final VoidCallback onTap;

  @override
  Widget build(BuildContext context) {
    return Semantics(
      button: true,
      label: label,
      excludeSemantics: true,
      child: InkWell(
        onTap: onTap,
        child: SizedBox(
          width: kMinTouchTarget,
          height: kMinTouchTarget,
          child: Icon(icon, size: 20, color: DokColors.ink),
        ),
      ),
    );
  }
}

/// The category strip. Horizontally scrollable, which is how a real shop
/// handles more categories than fit.
///
/// The strip is a filter, not decoration: tapping an entry re-queries the
/// catalogue with that category, and the entry that is applied is marked. A
/// control that looks selectable but does nothing is worse than no control
/// at all.
class _CategoryNav extends StatelessWidget {
  const _CategoryNav({
    required this.selectedCategory,
    required this.onSelect,
  });

  /// The category the catalogue is currently filtered by; `null` means
  /// "everything", which is how the strip opens.
  final String? selectedCategory;

  /// Called with the category id of the tapped entry.
  final ValueChanged<String> onSelect;

  /// (category id, English label, Japanese label).
  ///
  /// The id is the one the catalogue stores on a product and the one the
  /// server filters on (`categoryId` on `GET /v1/products`).
  static const categories = <(String, String, String)>[
    ('cat-lingerie', 'LINGERIE', 'ランジェリー'),
    ('cat-bra-sets', 'BRA & SETS', 'ブラ＆セット'),
    ('cat-nightwear', 'NIGHTWEAR', 'ナイトウェア'),
    ('cat-swim', 'SWIM', 'スイム'),
    ('cat-beauty', 'BEAUTY', 'ボディケア'),
    ('cat-goods', 'GOODS', 'グッズ'),
    ('cat-sale', 'SALE', 'セール'),
  ];

  /// The label of [categoryId], for the "nothing in this category" state.
  static String labelOf(String categoryId) {
    for (final (id, en, _) in categories) {
      if (id == categoryId) return en;
    }
    return categoryId;
  }

  @override
  Widget build(BuildContext context) {
    return Container(
      color: DokColors.surface,
      padding: const EdgeInsets.only(bottom: DokSpace.md),
      child: Column(
        children: [
          const Divider(height: 1),
          SizedBox(
            height: 44,
            child: ListView.separated(
              scrollDirection: Axis.horizontal,
              padding: EdgeInsets.symmetric(horizontal: DokGutter.of(MediaQuery.sizeOf(context).width)),
              itemCount: categories.length,
              separatorBuilder: (_, _) => const SizedBox(width: DokSpace.xl),
              itemBuilder: (context, index) {
                final (id, en, ja) = categories[index];
                final selected = id == selectedCategory;
                return Center(
                  child: Semantics(
                    button: true,
                    selected: selected,
                    label: '$ja / $en',
                    excludeSemantics: true,
                    child: InkWell(
                      onTap: () => onSelect(id),
                      child: Column(
                        mainAxisSize: MainAxisSize.min,
                        children: [
                          Text(
                            en,
                            style: Theme.of(context).textTheme.labelMedium?.copyWith(
                                  color: selected ? DokColors.crimson : DokColors.ink,
                                  letterSpacing: 1.4,
                                  fontWeight:
                                      selected ? FontWeight.w700 : FontWeight.w600,
                                ),
                          ),
                          // A hairline under the applied entry. The strip
                          // stays quiet, but the shopper can still see which
                          // filter is in force.
                          Container(
                            height: 2,
                            margin: const EdgeInsets.only(top: 4),
                            color:
                                selected ? DokColors.crimson : Colors.transparent,
                          ),
                        ],
                      ),
                    ),
                  ),
                );
              },
            ),
          ),
          const Divider(height: 1),
        ],
      ),
    );
  }
}

/// The editorial band under the nav: a statement, not a slideshow.
///
/// A first-tier fashion storefront opens with a message, not a carousel. The
/// reference site runs a topics strip of editorial links; this is the same
/// idea in one line.
class _HeroBanner extends StatelessWidget {
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
  final bool hasCategoryFilter;

  @override
  Widget build(BuildContext context) {
    final width = MediaQuery.sizeOf(context).width;

    return Container(
      width: double.infinity,
      color: DokColors.surface,
      padding: const EdgeInsets.symmetric(vertical: DokSpace.xxxl),
      child: _ContentWidth(
        child: Column(
        crossAxisAlignment: CrossAxisAlignment.start,
        children: [
          Text(
            'THE ART OF\nLINGERIE',
            style: Theme.of(context).textTheme.displaySmall?.copyWith(
                  fontSize: width < 600 ? 34 : 52,
                  height: 1.08,
                  letterSpacing: 1.5,
                ),
          ),
          const SizedBox(height: DokSpace.lg),
          SizedBox(
            width: 420,
            child: Text(
              '自分らしく着るための、下着の設計。'
              'Lingerie engineered to be worn, not to be looked at.',
              style: Theme.of(context).textTheme.bodyMedium?.copyWith(
                    color: DokColors.inkSoft,
                    height: 1.8,
                  ),
            ),
          ),
          const SizedBox(height: DokSpace.xl),
          OutlinedButton(
            // Nothing filtered means nothing to clear: the button says so by
            // being disabled, rather than being a control that is painted,
            // announced as a button, and does nothing.
            onPressed: hasCategoryFilter ? onShopAll : null,
            child: const Text('SHOP COLLECTION'),
          ),
        ],
        ),
      ),
    );
  }
}

/// A section title with a hairline rule — the reference store's section
/// headers are small caps with a rule, not large display type.
class _SectionRule extends StatelessWidget {
  const _SectionRule(this.title);

  final String title;

  @override
  Widget build(BuildContext context) {
    final gutter = DokGutter.of(MediaQuery.sizeOf(context).width);
    return Padding(
      padding: EdgeInsets.fromLTRB(gutter, DokSpace.xxl, gutter, 0),
      child: Row(
        children: [
          Text(
            title,
            style: Theme.of(context).textTheme.labelLarge?.copyWith(
                  letterSpacing: 2.4,
                  fontWeight: FontWeight.w600,
                ),
          ),
          const SizedBox(width: DokSpace.lg),
          const Expanded(child: Divider(height: 1)),
        ],
      ),
    );
  }
}

/// An inline notice, used for the guest-mode and degraded-channel messages.
class _Notice extends StatelessWidget {
  const _Notice({required this.message, this.actionLabel, this.onAction});

  final String message;
  final String? actionLabel;
  final VoidCallback? onAction;

  @override
  Widget build(BuildContext context) {
    final gutter = DokGutter.of(MediaQuery.sizeOf(context).width);
    return Padding(
      padding: EdgeInsets.fromLTRB(gutter, DokSpace.md, gutter, 0),
      child: StatusBanner(
        message: message,
        tone: BannerTone.info,
        actionLabel: actionLabel,
        onAction: onAction,
      ),
    );
  }
}

/// The product detail sheet.
class _ProductDetailSheet extends StatelessWidget {
  const _ProductDetailSheet({
    required this.product,
    required this.onAdd,
    this.isMutating = false,
    this.error,
  });

  final ProductSummary product;

  /// Adds the product to the cart. The sheet's button used to only close the
  /// sheet, so a shopper could believe they had added an item when they had
  /// not.
  final VoidCallback onAdd;
  final bool isMutating;
  final ApiProblem? error;

  @override
  Widget build(BuildContext context) {
    final theme = Theme.of(context);
    return Padding(
      padding: const EdgeInsets.fromLTRB(DokSpace.lg, 0, DokSpace.lg, DokSpace.xl),
      child: Column(
        mainAxisSize: MainAxisSize.min,
        crossAxisAlignment: CrossAxisAlignment.start,
        children: [
          Text(product.title, style: theme.textTheme.headlineSmall),
          const SizedBox(height: DokSpace.sm),
          PriceRow(money: product.price),
          const SizedBox(height: DokSpace.lg),
          AvailabilityBadge(availability: product.availability),
          const SizedBox(height: DokSpace.lg),
          const Divider(height: 1),
          const SizedBox(height: DokSpace.md),
          Text('SKU: ${product.sku}',
              style: theme.textTheme.bodySmall),
          const SizedBox(height: DokSpace.sm),
          Text(
            '価格はサーバー側で決定されます / The charged price is set server-side.',
            style: theme.textTheme.bodySmall,
          ),
          const SizedBox(height: DokSpace.xl),
          if (error != null) ...[
            StatusBanner(
              message: error!.detail.isEmpty
                  ? 'カートに追加できませんでした / Could not add to the cart.'
                  : error!.detail,
              tone: BannerTone.danger,
            ),
            const SizedBox(height: DokSpace.md),
          ],
          SizedBox(
            width: double.infinity,
            child: FilledButton(
              onPressed: product.availability == Availability.outOfStock || isMutating
                  ? null
                  : onAdd,
              child: Text(
                product.availability == Availability.outOfStock
                    ? '在庫切れ / Out of stock'
                    : isMutating
                        ? '追加中… / Adding…'
                        : 'カートに追加 / Add to cart',
              ),
            ),
          ),
        ],
      ),
    );
  }
}

/// The footer: the reassurance row a fashion store needs.
class _Footer extends StatelessWidget {
  const _Footer();

  static const promises = <(IconData, String)>[
    (Icons.local_shipping_outlined, '送料無料 / Free shipping over ¥10,000'),
    (Icons.autorenew, '返品・交換OK / Easy returns'),
    (Icons.lock_outline, '安全な決済 / Secure payment'),
  ];

  @override
  Widget build(BuildContext context) {
    final width = MediaQuery.sizeOf(context).width;
    final gutter = DokGutter.of(width);

    return Container(
      width: double.infinity,
      color: DokColors.surface,
      margin: EdgeInsets.only(top: DokSpace.huge),
      padding: EdgeInsets.symmetric(vertical: DokSpace.xxl, horizontal: gutter),
      child: Column(
        children: [
          Wrap(
            alignment: WrapAlignment.center,
            spacing: DokSpace.xxl,
            runSpacing: DokSpace.md,
            children: [
              for (final (icon, label) in promises)
                Row(
                  mainAxisSize: MainAxisSize.min,
                  children: [
                    Icon(icon, size: 18, color: DokColors.inkSoft),
                    const SizedBox(width: DokSpace.sm),
                    Text(
                      label,
                      style: Theme.of(context).textTheme.labelMedium,
                    ),
                  ],
                ),
            ],
          ),
          const SizedBox(height: DokSpace.xxl),
          const Divider(height: 1),
          const SizedBox(height: DokSpace.lg),
          Text(
            'DOKIPON',
            style: Theme.of(context).textTheme.titleMedium?.copyWith(
                  fontFamily: DokTheme.displayFamily,
                  letterSpacing: 5,
                ),
          ),
          const SizedBox(height: DokSpace.sm),
          Text(
            '© 2026 DOKIPON',
            style: Theme.of(context).textTheme.bodySmall,
          ),
        ],
      ),
    );
  }
}

/// Shows live checkout status at the bottom of the catalogue.
///
/// The wording for `payment_unknown` is the important part: it must not say
/// "failed", and it must not invite the user to pay again.
class CheckoutStatusBar extends StatelessWidget {
  const CheckoutStatusBar({super.key, required this.checkout});

  final CheckoutController checkout;

  static (String, BannerTone) describe(CheckoutStatus status) => switch (status) {
        CheckoutStatus.confirmed => (
            'ご注文を完了しました / Order confirmed.',
            BannerTone.success
          ),
        CheckoutStatus.rejected => (
            'ご注文を処理できませんでした / We could not process this order.',
            BannerTone.danger
          ),
        CheckoutStatus.paymentUnknown => (
            'お支払いを確認中です。Charged at most once — '
            'we are confirming with your payment provider.',
            BannerTone.warning
          ),
        CheckoutStatus.cancelled => (
            'ご注文はキャンセルされました / Order cancelled.',
            BannerTone.warning
          ),
        CheckoutStatus.unknown => (
            'ステータス不明 — 注文一覧をご確認ください / '
            'Status unknown — please check your orders.',
            BannerTone.warning
          ),
        _ => (
            '処理中… / Processing…',
            BannerTone.info
          ),
      };

  @override
  Widget build(BuildContext context) {
    if (checkout.snapshot == null) return const SizedBox.shrink();
    final (message, tone) = describe(checkout.status);
    final detail = checkout.error?.detail ?? '';

    return SafeArea(
      child: Padding(
        padding: const EdgeInsets.all(DokSpace.md),
        child: StatusBanner(
          message: detail.isNotEmpty ? '$detail\n$message' : message,
          tone: tone,
          actionLabel: checkout.status.isPending ? '更新 / Refresh' : null,
          // A refresh re-reads status; it never re-submits the checkout.
          onAction: checkout.status.isPending ? checkout.poll : null,
        ),
      ),
    );
  }
}