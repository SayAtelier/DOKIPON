/*
@cypher schema=1 source_sha256=98e1a4a9ecd01d268c7de0e016c035c9977a78e2aa60622da71d0b6ec6cec03f
MERGE (self:File {path:"apps/admin_web/lib/screens/orders_screen.dart"})
MERGE (screen:Symbol {id:"apps/admin_web/lib/screens/orders_screen.dart::OrdersScreen", kind:"class"})
MERGE (state:Symbol {id:"apps/admin_web/lib/screens/orders_screen.dart::_OrdersScreenState", kind:"class"})
MERGE (format:Symbol {id:"apps/admin_web/lib/screens/orders_screen.dart::_formatDate", kind:"function"})
MERGE (controller:Symbol {id:"packages/storefront_state/lib/src/admin_console_controller.dart::AdminConsoleController", kind:"class"})
MERGE (price:Symbol {id:"packages/design_system/lib/src/widgets/price_label.dart::PriceLabel", kind:"class"})
MERGE (chip:Symbol {id:"apps/admin_web/lib/widgets/admin_state_chip.dart::AdminStateChip", kind:"class"})
MERGE (failure:Symbol {id:"apps/admin_web/lib/widgets/load_failure.dart::LoadFailure", kind:"class"})
MERGE (self)-[:DEFINES]->(screen)
MERGE (self)-[:DEFINES]->(state)
MERGE (screen)-[:USES_TYPE]->(controller)
MERGE (state)-[:CALLS]->(format)
MERGE (state)-[:CREATES]->(price)
MERGE (state)-[:CREATES]->(chip)
MERGE (state)-[:CREATES]->(failure)
@endcypher
*/
import 'dart:async';

import 'package:design_system/design_system.dart';
import 'package:domain_models/domain_models.dart';
import 'package:flutter/material.dart';
import 'package:storefront_state/storefront_state.dart';

import '../widgets/admin_state_chip.dart';
import '../widgets/load_failure.dart';

class OrdersScreen extends StatefulWidget {
  const OrdersScreen({
    super.key,
    required this.controller,
    required this.onOpenOrder,
  });

  final AdminConsoleController controller;
  final ValueChanged<String> onOpenOrder;

  @override
  State<OrdersScreen> createState() => _OrdersScreenState();
}

class _OrdersScreenState extends State<OrdersScreen> {
  @override
  void initState() {
    super.initState();
    if (!widget.controller.hasLoadedOrders &&
        widget.controller.ordersError == null) {
      unawaited(widget.controller.loadOrders());
    }
  }

  @override
  Widget build(BuildContext context) => AnimatedBuilder(
        animation: widget.controller,
        builder: (context, _) {
          final controller = widget.controller;
          if (controller.isOrdersLoading && !controller.hasLoadedOrders) {
            return const Center(child: CircularProgressIndicator());
          }
          if (controller.ordersError != null && !controller.hasLoadedOrders) {
            return LoadFailure(
              message: controller.ordersError!,
              onRetry: () => unawaited(controller.loadOrders()),
            );
          }
          return Padding(
            padding: const EdgeInsets.all(GemSpace.lg),
            child: Column(
              crossAxisAlignment: CrossAxisAlignment.stretch,
              children: [
                Row(
                  children: [
                    Expanded(
                      child: Column(
                        crossAxisAlignment: CrossAxisAlignment.start,
                        children: [
                          Text('Orders',
                              style: Theme.of(context).textTheme.headlineMedium),
                          const SizedBox(height: GemSpace.xs),
                          const Text('Review commerce activity and payment state.'),
                        ],
                      ),
                    ),
                    IconButton(
                      tooltip: 'Refresh orders',
                      onPressed: controller.isOrdersLoading
                          ? null
                          : () => unawaited(controller.loadOrders()),
                      icon: const Icon(Icons.refresh),
                    ),
                  ],
                ),
                const SizedBox(height: GemSpace.md),
                Row(
                  children: [
                    Text('State', style: Theme.of(context).textTheme.labelLarge),
                    const SizedBox(width: GemSpace.sm),
                    DropdownButton<CheckoutStatus?>(
                      value: controller.orderFilter,
                      onChanged: controller.setOrderFilter,
                      items: [
                        const DropdownMenuItem<CheckoutStatus?>(
                          value: null,
                          child: Text('All states'),
                        ),
                        ...CheckoutStatus.values.map(
                          (status) => DropdownMenuItem<CheckoutStatus?>(
                            value: status,
                            child: Text(AdminStateChip.labelFor(status)),
                          ),
                        ),
                      ],
                    ),
                    const Spacer(),
                    Text('${controller.orders.length} orders'),
                  ],
                ),
                if (controller.isOrdersLoading) const LinearProgressIndicator(),
                if (controller.ordersError case final error?) ...[
                  const SizedBox(height: GemSpace.sm),
                  StatusBanner(message: error, tone: BannerTone.danger),
                  Align(
                    alignment: Alignment.centerRight,
                    child: TextButton.icon(
                      onPressed: () => unawaited(controller.loadOrders()),
                      icon: const Icon(Icons.refresh),
                      label: const Text('Retry'),
                    ),
                  ),
                ],
                const SizedBox(height: GemSpace.sm),
                Expanded(
                  child: controller.orders.isEmpty
                      ? const Center(child: Text('No orders match this state.'))
                      : Card(
                          child: SingleChildScrollView(
                            scrollDirection: Axis.horizontal,
                            child: SingleChildScrollView(
                              child: DataTable(
                                columns: const [
                                  DataColumn(label: Text('Order')),
                                  DataColumn(label: Text('State')),
                                  DataColumn(label: Text('Total')),
                                  DataColumn(label: Text('Created at')),
                                  DataColumn(label: Text('')),
                                ],
                                rows: controller.orders.map((order) {
                                  return DataRow(
                                    cells: [
                                      DataCell(Text(order.orderId)),
                                      DataCell(AdminStateChip(status: order.state)),
                                      DataCell(PriceLabel(money: order.total)),
                                      DataCell(Text(_formatDate(order.createdAt))),
                                      DataCell(
                                        TextButton.icon(
                                          onPressed: () =>
                                              widget.onOpenOrder(order.orderId),
                                          icon: const Icon(Icons.open_in_new),
                                          label: const Text('Open'),
                                        ),
                                      ),
                                    ],
                                  );
                                }).toList(growable: false),
                              ),
                            ),
                          ),
                        ),
                ),
              ],
            ),
          );
        },
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
