/*
@cypher schema=1 source_sha256=84820047d01b99128bb9c4398c86548eee04502eada7f6045aed2865dd7e2063
MERGE (self:File {path:"apps/admin_web/lib/screens/order_detail_screen.dart"})
MERGE (screen:Symbol {id:"apps/admin_web/lib/screens/order_detail_screen.dart::OrderDetailScreen", kind:"class"})
MERGE (line:Symbol {id:"apps/admin_web/lib/screens/order_detail_screen.dart::_OrderLines", kind:"class"})
MERGE (trail:Symbol {id:"apps/admin_web/lib/screens/order_detail_screen.dart::_OrderTrail", kind:"class"})
MERGE (controller:Symbol {id:"packages/storefront_state/lib/src/admin_console_controller.dart::AdminConsoleController", kind:"class"})
MERGE (price:Symbol {id:"packages/design_system/lib/src/widgets/price_label.dart::PriceLabel", kind:"class"})
MERGE (chip:Symbol {id:"apps/admin_web/lib/widgets/admin_state_chip.dart::AdminStateChip", kind:"class"})
MERGE (reconcile:Symbol {id:"apps/admin_web/lib/widgets/reconcile_actions.dart::ReconcileActions", kind:"class"})
MERGE (failure:Symbol {id:"apps/admin_web/lib/widgets/load_failure.dart::LoadFailure", kind:"class"})
MERGE (self)-[:DEFINES]->(screen)
MERGE (self)-[:DEFINES]->(line)
MERGE (self)-[:DEFINES]->(trail)
MERGE (screen)-[:USES_TYPE]->(controller)
MERGE (screen)-[:CREATES]->(line)
MERGE (screen)-[:CREATES]->(trail)
MERGE (screen)-[:CREATES]->(price)
MERGE (screen)-[:CREATES]->(chip)
MERGE (screen)-[:CREATES]->(reconcile)
MERGE (screen)-[:CREATES]->(failure)
@endcypher
*/
import 'dart:async';

import 'package:design_system/design_system.dart';
import 'package:domain_models/domain_models.dart';
import 'package:flutter/material.dart';
import 'package:storefront_state/storefront_state.dart';

import '../widgets/admin_state_chip.dart';
import '../widgets/load_failure.dart';
import '../widgets/reconcile_actions.dart';

class OrderDetailScreen extends StatelessWidget {
  const OrderDetailScreen({
    super.key,
    required this.controller,
    required this.onBack,
  });

  final AdminConsoleController controller;
  final VoidCallback onBack;

  @override
  Widget build(BuildContext context) => AnimatedBuilder(
        animation: controller,
        builder: (context, _) {
          final order = controller.selectedOrder;
          if (controller.isOrderLoading && order == null) {
            return const Center(child: CircularProgressIndicator());
          }
          if (order == null) {
            final message = controller.orderError;
            if (message != null) {
              return LoadFailure(
                message: message,
                onRetry: () {
                  final id = controller.selectedOrderId;
                  if (id != null) unawaited(controller.loadOrderDetail(id));
                },
              );
            }
            return const Center(child: Text('Select an order to view its details.'));
          }

          return Padding(
            padding: const EdgeInsets.all(GemSpace.lg),
            child: ListView(
              children: [
                Row(
                  children: [
                    IconButton(
                      tooltip: 'Back to orders',
                      onPressed: onBack,
                      icon: const Icon(Icons.arrow_back),
                    ),
                    Expanded(
                      child: Text('Order ${order.orderId}',
                          style: Theme.of(context).textTheme.headlineMedium),
                    ),
                    AdminStateChip(status: order.state),
                  ],
                ),
                if (controller.isOrderLoading) const LinearProgressIndicator(),
                if (controller.orderError case final error?) ...[
                  const SizedBox(height: GemSpace.md),
                  StatusBanner(message: error, tone: BannerTone.danger),
                  Align(
                    alignment: Alignment.centerRight,
                    child: TextButton.icon(
                      onPressed: () {
                        final id = controller.selectedOrderId;
                        if (id != null) unawaited(controller.loadOrderDetail(id));
                      },
                      icon: const Icon(Icons.refresh),
                      label: const Text('Retry'),
                    ),
                  ),
                ],
                const SizedBox(height: GemSpace.md),
                Card(
                  child: Padding(
                    padding: const EdgeInsets.all(GemSpace.md),
                    child: Wrap(
                      spacing: GemSpace.xl,
                      runSpacing: GemSpace.md,
                      children: [
                        _OrderMetric(label: 'Total', child: PriceLabel(money: order.total)),
                        _OrderMetric(
                          label: 'Created at',
                          child: Text(_formatDate(order.createdAt)),
                        ),
                        _OrderMetric(
                          label: 'Payment state',
                          child: Text(order.state == CheckoutStatus.paymentUnknown
                              ? 'Needs review'
                              : AdminStateChip.labelFor(order.state)),
                        ),
                      ],
                    ),
                  ),
                ),
                const SizedBox(height: GemSpace.md),
                _OrderLines(lines: order.lines),
                const SizedBox(height: GemSpace.md),
                _OrderTrail(entries: order.trail),
                const SizedBox(height: GemSpace.md),
                ReconcileActions(
                  status: order.state,
                  isWorking: controller.isReconciling,
                  onMarkAuthorized: () =>
                      unawaited(controller.reconcile(authorized: true)),
                  onMarkDeclined: () =>
                      unawaited(controller.reconcile(authorized: false)),
                ),
              ],
            ),
          );
        },
      );
}

class _OrderMetric extends StatelessWidget {
  const _OrderMetric({required this.label, required this.child});

  final String label;
  final Widget child;

  @override
  Widget build(BuildContext context) => SizedBox(
        width: 220,
        child: Column(
          crossAxisAlignment: CrossAxisAlignment.start,
          children: [
            Text(label, style: Theme.of(context).textTheme.labelMedium),
            const SizedBox(height: GemSpace.xs),
            child,
          ],
        ),
      );
}

class _OrderLines extends StatelessWidget {
  const _OrderLines({required this.lines});

  final List<OrderLine> lines;

  @override
  Widget build(BuildContext context) => Card(
        child: Padding(
          padding: const EdgeInsets.all(GemSpace.md),
          child: Column(
            crossAxisAlignment: CrossAxisAlignment.start,
            children: [
              Text('Line items', style: Theme.of(context).textTheme.titleLarge),
              const SizedBox(height: GemSpace.sm),
              if (lines.isEmpty)
                const Text('No line items were returned for this order.')
              else
                SingleChildScrollView(
                  scrollDirection: Axis.horizontal,
                  child: DataTable(
                    columns: const [
                      DataColumn(label: Text('SKU')),
                      DataColumn(label: Text('Quantity'), numeric: true),
                      DataColumn(label: Text('Unit price'), numeric: true),
                      DataColumn(label: Text('Line total'), numeric: true),
                    ],
                    rows: lines
                        .map((line) => DataRow(cells: [
                              DataCell(Text(line.sku)),
                              DataCell(Text('${line.quantity}')),
                              DataCell(PriceLabel(money: line.unitPrice)),
                              DataCell(PriceLabel(money: line.lineTotal)),
                            ]))
                        .toList(growable: false),
                  ),
                ),
            ],
          ),
        ),
      );
}

class _OrderTrail extends StatelessWidget {
  const _OrderTrail({required this.entries});

  final List<AdminOrderTrailEntry> entries;

  @override
  Widget build(BuildContext context) => Card(
        child: Padding(
          padding: const EdgeInsets.all(GemSpace.md),
          child: Column(
            crossAxisAlignment: CrossAxisAlignment.start,
            children: [
              Text('State trail', style: Theme.of(context).textTheme.titleLarge),
              const SizedBox(height: GemSpace.sm),
              if (entries.isEmpty)
                const Text('No state transitions were returned for this order.')
              else
                ...entries.map((entry) => ListTile(
                      contentPadding: EdgeInsets.zero,
                      leading: CircleAvatar(child: Text('${entry.sequence}')),
                      title: Text(
                        '${AdminStateChip.labelFor(entry.from)}  →  '
                        '${AdminStateChip.labelFor(entry.to)}',
                      ),
                      subtitle: Text(entry.event.isEmpty ? 'State changed' : entry.event),
                      trailing: entry.occurredAt == null
                          ? null
                          : Text(_formatDate(entry.occurredAt)),
                    )),
            ],
          ),
        ),
      );
}

String _formatDate(DateTime? value) {
  if (value == null) return '—';
  final date = value.toLocal();
  final month = date.month.toString().padLeft(2, '0');
  final day = date.day.toString().padLeft(2, '0');
  final hour = date.hour.toString().padLeft(2, '0');
  final minute = date.minute.toString().padLeft(2, '0');
  return '${date.year}-$month-$day $hour:$minute';
}
