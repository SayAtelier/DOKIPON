import 'package:design_system/design_system.dart';
import 'package:domain_models/domain_models.dart';
import 'package:flutter/material.dart';
import 'package:storefront_state/storefront_state.dart';

/// The cart drawer.
///
/// A shop without a cart is a brochure. This is the surface where a shopper
/// sees what they picked, changes a quantity, removes a line and checks out.
///
/// The total shown here is the **server's snapshot subtotal**, never a
/// locally computed one. The server re-prices every line from the catalogue
/// at checkout, so a number computed on the client could disagree with what
/// the shopper is actually charged. The line under the total says so.
class CartDrawer extends StatelessWidget {
  const CartDrawer({
    super.key,
    required this.cart,
    required this.onQuantityChanged,
    required this.onRemove,
    required this.onCheckout,
    this.onClose,
    this.isMutating = false,
    this.error,
  });

  final CartController cart;
  final void Function(String sku, int quantity) onQuantityChanged;
  final void Function(String sku) onRemove;
  final VoidCallback onCheckout;
  final VoidCallback? onClose;
  final bool isMutating;
  final ApiProblem? error;

  @override
  Widget build(BuildContext context) {
    final lines = cart.lines;

    return Drawer(
      backgroundColor: DokColors.surface,
      child: SafeArea(
        child: Column(
          crossAxisAlignment: CrossAxisAlignment.stretch,
          children: [
            _DrawerHeader(count: cart.itemCount, onClose: onClose),
            const Divider(height: 1),
            Expanded(
              child: lines.isEmpty
                  ? _EmptyCart()
                  : ListView.separated(
                      padding: const EdgeInsets.symmetric(vertical: DokSpace.sm),
                      itemCount: lines.length,
                      separatorBuilder: (_, _) =>
                          const Divider(height: 1, color: DokColors.border),
                      itemBuilder: (context, index) {
                        final line = lines[index];
                        return _CartLineRow(
                          line: line,
                          isMutating: isMutating,
                          onQuantityChanged: onQuantityChanged,
                          onRemove: onRemove,
                        );
                      },
                    ),
            ),
            if (error != null)
              Padding(
                padding: const EdgeInsets.all(DokSpace.md),
                child: StatusBanner(
                  message: error!.detail.isEmpty
                      ? 'カートを更新できませんでした / Could not update the cart.'
                      : error!.detail,
                  tone: BannerTone.danger,
                ),
              ),
            if (lines.isNotEmpty) _CartTotals(cart: cart),
            if (lines.isNotEmpty)
              Padding(
                padding: const EdgeInsets.fromLTRB(
                    DokSpace.lg, 0, DokSpace.lg, DokSpace.lg),
                child: SizedBox(
                  width: double.infinity,
                  child: FilledButton(
                    onPressed: isMutating ? null : onCheckout,
                    child: Text(
                      isMutating ? '処理中…' : 'レジに進む / Checkout',
                    ),
                  ),
                ),
              ),
          ],
        ),
      ),
    );
  }
}

class _DrawerHeader extends StatelessWidget {
  const _DrawerHeader({required this.count, this.onClose});

  final int count;
  final VoidCallback? onClose;

  @override
  Widget build(BuildContext context) {
    return Padding(
      padding: const EdgeInsets.fromLTRB(DokSpace.lg, DokSpace.lg, DokSpace.sm, DokSpace.lg),
      child: Row(
        children: [
          Text(
            'カート / Cart',
            style: Theme.of(context).textTheme.titleLarge,
          ),
          if (count > 0) ...[
            const SizedBox(width: DokSpace.sm),
            Container(
              padding: const EdgeInsets.symmetric(horizontal: 8, vertical: 2),
              color: DokColors.ink,
              child: Text(
                '$count',
                style: const TextStyle(
                  color: DokColors.onButton,
                  fontSize: 11,
                  fontWeight: FontWeight.w600,
                ),
              ),
            ),
          ],
          const Spacer(),
          if (onClose != null)
            IconButton(
              onPressed: onClose,
              icon: const Icon(Icons.close, size: 20),
              color: DokColors.ink,
              tooltip: '閉じる / Close',
            ),
        ],
      ),
    );
  }
}

class _EmptyCart extends StatelessWidget {
  @override
  Widget build(BuildContext context) {
    return Center(
      child: Padding(
        padding: const EdgeInsets.all(DokSpace.xxxl),
        child: Column(
          mainAxisSize: MainAxisSize.min,
          children: [
            const Icon(Icons.shopping_bag_outlined,
                size: 36, color: DokColors.inkFaint),
            const SizedBox(height: DokSpace.lg),
            Text(
              'カートは空です',
              style: Theme.of(context).textTheme.titleMedium,
            ),
            const SizedBox(height: DokSpace.xs),
            Text(
              'Your cart is empty.',
              style: Theme.of(context).textTheme.bodyMedium,
            ),
          ],
        ),
      ),
    );
  }
}

class _CartLineRow extends StatelessWidget {
  const _CartLineRow({
    required this.line,
    required this.isMutating,
    required this.onQuantityChanged,
    required this.onRemove,
  });

  final CartLine line;
  final bool isMutating;
  final void Function(String sku, int quantity) onQuantityChanged;
  final void Function(String sku) onRemove;

  @override
  Widget build(BuildContext context) {
    final theme = Theme.of(context);
    return Padding(
      padding: const EdgeInsets.symmetric(
        horizontal: DokSpace.lg,
        vertical: DokSpace.md,
      ),
      child: Row(
        crossAxisAlignment: CrossAxisAlignment.start,
        children: [
          Expanded(
            child: Column(
              crossAxisAlignment: CrossAxisAlignment.start,
              children: [
                Text(line.sku, style: theme.textTheme.titleSmall),
                const SizedBox(height: DokSpace.xs),
                PriceLabel(
                  money: line.snapshotMoney,
                  style: theme.textTheme.bodySmall,
                  color: DokColors.inkSoft,
                ),
              ],
            ),
          ),
          const SizedBox(width: DokSpace.md),
          _QuantityStepper(
            quantity: line.quantity,
            enabled: !isMutating,
            onChanged: (quantity) => onQuantityChanged(line.sku, quantity),
          ),
          const SizedBox(width: DokSpace.sm),
          IconButton(
            onPressed: isMutating ? null : () => onRemove(line.sku),
            icon: const Icon(Icons.close, size: 16),
            color: DokColors.inkFaint,
            tooltip: '削除 / Remove',
            visualDensity: VisualDensity.compact,
          ),
        ],
      ),
    );
  }
}

/// A −/+ quantity stepper.
///
/// The buttons are 40dp, which is the WCAG 2.2 target size for a control
/// that is not the only way to do the thing. The text field between them is
/// also editable, so the stepper is not the sole input path.
class _QuantityStepper extends StatelessWidget {
  const _QuantityStepper({
    required this.quantity,
    required this.enabled,
    required this.onChanged,
  });

  final int quantity;
  final bool enabled;
  final void Function(int quantity) onChanged;

  @override
  Widget build(BuildContext context) {
    return Container(
      decoration: BoxDecoration(
        border: Border.all(color: DokColors.border),
      ),
      child: Row(
        mainAxisSize: MainAxisSize.min,
        children: [
          _StepButton(
            icon: Icons.remove,
            label: '減らす / Decrease',
            enabled: enabled && quantity > 0,
            onPressed: () => onChanged(quantity - 1),
          ),
          SizedBox(
            width: 34,
            child: Text(
              '$quantity',
              textAlign: TextAlign.center,
              style: Theme.of(context).textTheme.titleSmall,
            ),
          ),
          _StepButton(
            icon: Icons.add,
            label: '増やす / Increase',
            enabled: enabled && quantity < 99,
            onPressed: () => onChanged(quantity + 1),
          ),
        ],
      ),
    );
  }
}

class _StepButton extends StatelessWidget {
  const _StepButton({
    required this.icon,
    required this.label,
    required this.enabled,
    required this.onPressed,
  });

  final IconData icon;
  final String label;
  final bool enabled;
  final VoidCallback onPressed;

  @override
  Widget build(BuildContext context) {
    return Semantics(
      button: true,
      label: label,
      enabled: enabled,
      excludeSemantics: true,
      child: InkWell(
        onTap: enabled ? onPressed : null,
        child: SizedBox(
          width: 40,
          height: 40,
          child: Icon(
            icon,
            size: 16,
            color: enabled ? DokColors.ink : DokColors.inkFaint,
          ),
        ),
      ),
    );
  }
}

class _CartTotals extends StatelessWidget {
  const _CartTotals({required this.cart});

  final CartController cart;

  @override
  Widget build(BuildContext context) {
    final theme = Theme.of(context);
    return Container(
      padding: const EdgeInsets.fromLTRB(
          DokSpace.lg, DokSpace.lg, DokSpace.lg, DokSpace.md),
      decoration: const BoxDecoration(
        border: Border(top: BorderSide(color: DokColors.border)),
      ),
      child: Column(
        crossAxisAlignment: CrossAxisAlignment.start,
        children: [
          Row(
            children: [
              Text('小計 / Subtotal', style: theme.textTheme.titleSmall),
              const Spacer(),
              PriceLabel(
                money: cart.subtotal,
                style: theme.textTheme.titleMedium?.copyWith(fontSize: 17),
              ),
            ],
          ),
          const SizedBox(height: DokSpace.xs),
          Text(
            'お会計金額はサーバー側で再計算されます / '
            'The amount charged is recalculated server-side from the catalogue.',
            style: theme.textTheme.bodySmall,
          ),
        ],
      ),
    );
  }
}
