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
  late final CheckoutController checkout;
  late StorefrontConfig config;
  bool _booting = true;

  @override
  void initState() {
    super.initState();
    session = SessionController(api: widget.api);
    catalog = CatalogController(widget.api);
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
    if (mounted) {
      setState(() => _booting = false);
    }
  }

  @override
  void dispose() {
    session.dispose();
    catalog.dispose();
    checkout.dispose();
    super.dispose();
  }

  @override
  Widget build(BuildContext context) {
    final seed = widget.environment.channel == StoreChannel.line
        ? GemColors.line
        : GemColors.brand;
    return MaterialApp(
      title: 'GemArc Commerce',
      debugShowCheckedModeBanner: false,
      theme: GemTheme.light(seed: seed),
      home: CatalogScreen(
        session: session,
        catalog: catalog,
        checkout: checkout,
        booting: _booting,
      ),
    );
  }
}

/// The catalogue screen, shared by every channel.
class CatalogScreen extends StatefulWidget {
  const CatalogScreen({
    super.key,
    required this.session,
    required this.catalog,
    required this.checkout,
    this.booting = false,
  });

  final SessionController session;
  final CatalogController catalog;
  final CheckoutController checkout;
  final bool booting;

  @override
  State<CatalogScreen> createState() => _CatalogScreenState();
}

class _CatalogScreenState extends State<CatalogScreen> {
  @override
  void initState() {
    super.initState();
    widget.session.addListener(_onChange);
    widget.catalog.addListener(_onChange);
    widget.checkout.addListener(_onChange);
  }

  @override
  void dispose() {
    widget.session.removeListener(_onChange);
    widget.catalog.removeListener(_onChange);
    widget.checkout.removeListener(_onChange);
    super.dispose();
  }

  void _onChange() {
    if (mounted) setState(() {});
  }

  @override
  Widget build(BuildContext context) {
    final catalog = widget.catalog;
    final session = widget.session;

    return Scaffold(
      appBar: AppBar(
        title: const Text('GemArc Commerce'),
        backgroundColor: GemColors.surface,
      ),
      body: RefreshIndicator(
        onRefresh: () => catalog.load(refresh: true),
        child: ListView(
          physics: const AlwaysScrollableScrollPhysics(),
          padding: const EdgeInsets.symmetric(vertical: GemSpace.sm),
          children: [
            if (session.notice != null)
              Padding(
                padding: const EdgeInsets.fromLTRB(
                    GemSpace.md, GemSpace.sm, GemSpace.md, 0),
                child: StatusBanner(
                  message: session.notice!,
                  tone: BannerTone.warning,
                  actionLabel: 'OK',
                  onAction: session.clearNotice,
                ),
              ),
            if (session.isGuest && !session.isBooting)
              const Padding(
                padding: EdgeInsets.fromLTRB(
                    GemSpace.md, GemSpace.sm, GemSpace.md, 0),
                child: StatusBanner(
                  message: 'ゲスト表示です。購入にはログインが必要です / '
                      'Browsing as a guest — sign in to check out.',
                  tone: BannerTone.info,
                ),
              ),
            const SizedBox(height: GemSpace.sm),
            if (widget.booting || (catalog.isLoading && catalog.items.isEmpty))
              const Padding(
                padding: EdgeInsets.all(GemSpace.xl),
                child: Center(child: CircularProgressIndicator()),
              )
            else if (catalog.error != null && catalog.items.isEmpty)
              Padding(
                padding: const EdgeInsets.all(GemSpace.md),
                child: StatusBanner(
                  message: catalog.error!.detail.isEmpty
                      ? '商品を読み込めませんでした / Could not load products.'
                      : catalog.error!.detail,
                  tone: BannerTone.danger,
                  actionLabel: '再試行 / Retry',
                  onAction: () => catalog.load(refresh: true),
                ),
              )
            else if (catalog.isEmpty)
              const Padding(
                padding: EdgeInsets.all(GemSpace.xl),
                child: Center(
                    child: Text('商品がありません / No products available')),
              )
            else
              for (final p in catalog.items)
                ProductCard(product: p, onOpen: () => _showDetail(p)),
            if (catalog.hasMore)
              Padding(
                padding: const EdgeInsets.all(GemSpace.md),
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
            const SizedBox(height: GemSpace.xl),
          ],
        ),
      ),
      bottomNavigationBar: CheckoutStatusBar(checkout: widget.checkout),
    );
  }

  void _showDetail(ProductSummary product) {
    showModalBottomSheet<void>(
      context: context,
      showDragHandle: true,
      builder: (_) => Padding(
        padding: const EdgeInsets.all(GemSpace.md),
        child: Column(
          mainAxisSize: MainAxisSize.min,
          crossAxisAlignment: CrossAxisAlignment.start,
          children: [
            Text(product.title, style: Theme.of(context).textTheme.titleLarge),
            const SizedBox(height: GemSpace.sm),
            PriceLabel(
              money: product.price,
              style: Theme.of(context).textTheme.headlineSmall,
            ),
            const SizedBox(height: GemSpace.md),
            AvailabilityBadge(availability: product.availability),
            const SizedBox(height: GemSpace.md),
            Text('SKU: ${product.sku}',
                style: Theme.of(context).textTheme.bodySmall),
            const SizedBox(height: GemSpace.sm),
            const Text(
              '価格はサーバー側で決定されます / The charged price is set server-side.',
              style: TextStyle(color: GemColors.inkMuted, fontSize: 12),
            ),
          ],
        ),
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
        padding: const EdgeInsets.all(GemSpace.md),
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
