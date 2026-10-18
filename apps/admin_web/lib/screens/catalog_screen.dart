/*
@cypher schema=1 source_sha256=0ef0127f3e9511d847932a5d75162d041344c2a5a88aa1ccad1d33bc78434fea
MERGE (self:File {path:"apps/admin_web/lib/screens/catalog_screen.dart"})
MERGE (screen:Symbol {id:"apps/admin_web/lib/screens/catalog_screen.dart::CatalogScreen", kind:"class"})
MERGE (state:Symbol {id:"apps/admin_web/lib/screens/catalog_screen.dart::_CatalogScreenState", kind:"class"})
MERGE (eligibility:Symbol {id:"apps/admin_web/lib/screens/catalog_screen.dart::_EligibilityCell", kind:"class"})
MERGE (controller:Symbol {id:"packages/storefront_state/lib/src/admin_console_controller.dart::AdminConsoleController", kind:"class"})
MERGE (price:Symbol {id:"packages/design_system/lib/src/widgets/price_label.dart::PriceLabel", kind:"class"})
MERGE (failure:Symbol {id:"apps/admin_web/lib/widgets/load_failure.dart::LoadFailure", kind:"class"})
MERGE (self)-[:DEFINES]->(screen)
MERGE (self)-[:DEFINES]->(state)
MERGE (self)-[:DEFINES]->(eligibility)
MERGE (screen)-[:USES_TYPE]->(controller)
MERGE (state)-[:CREATES]->(price)
MERGE (state)-[:CREATES]->(failure)
@endcypher
*/
import 'dart:async';

import 'package:design_system/design_system.dart';
import 'package:flutter/material.dart';
import 'package:storefront_state/storefront_state.dart';

import '../widgets/load_failure.dart';

class CatalogScreen extends StatefulWidget {
  const CatalogScreen({super.key, required this.controller});

  final AdminConsoleController controller;

  @override
  State<CatalogScreen> createState() => _CatalogScreenState();
}

class _CatalogScreenState extends State<CatalogScreen> {
  @override
  void initState() {
    super.initState();
    if (!widget.controller.hasLoadedCatalog &&
        widget.controller.catalogError == null) {
      unawaited(widget.controller.loadCatalog());
    }
  }

  @override
  Widget build(BuildContext context) => AnimatedBuilder(
        animation: widget.controller,
        builder: (context, _) {
          final controller = widget.controller;
          if (controller.isCatalogLoading && !controller.hasLoadedCatalog) {
            return const Center(child: CircularProgressIndicator());
          }
          if (controller.catalogError != null && !controller.hasLoadedCatalog) {
            return LoadFailure(
              message: controller.catalogError!,
              onRetry: () => unawaited(controller.loadCatalog()),
            );
          }
          return Padding(
            padding: const EdgeInsets.all(GemSpace.lg),
            child: Column(
              crossAxisAlignment: CrossAxisAlignment.stretch,
              children: [
                Text('Catalog', style: Theme.of(context).textTheme.headlineMedium),
                const SizedBox(height: GemSpace.xs),
                const Text('Product information and channel eligibility.'),
                const SizedBox(height: GemSpace.md),
                const StatusBanner(
                  message: 'Catalog is read only in v1. Product changes are managed in the commerce service.',
                  tone: BannerTone.info,
                ),
                if (controller.isCatalogLoading) const LinearProgressIndicator(),
                if (controller.catalogError case final error?) ...[
                  const SizedBox(height: GemSpace.sm),
                  StatusBanner(message: error, tone: BannerTone.danger),
                  Align(
                    alignment: Alignment.centerRight,
                    child: TextButton.icon(
                      onPressed: () => unawaited(controller.loadCatalog()),
                      icon: const Icon(Icons.refresh),
                      label: const Text('Retry'),
                    ),
                  ),
                ],
                const SizedBox(height: GemSpace.md),
                Expanded(
                  child: controller.catalog.isEmpty
                      ? const Center(child: Text('No products are available.'))
                      : Card(
                          child: SingleChildScrollView(
                            scrollDirection: Axis.horizontal,
                            child: SingleChildScrollView(
                              child: DataTable(
                                columns: const [
                                  DataColumn(label: Text('Product')),
                                  DataColumn(label: Text('SKU')),
                                  DataColumn(label: Text('Price')),
                                  DataColumn(label: Text('Web')),
                                  DataColumn(label: Text('App')),
                                  DataColumn(label: Text('LINE mini app')),
                                ],
                                rows: controller.catalog.map((product) {
                                  return DataRow(cells: [
                                    DataCell(Text(product.title)),
                                    DataCell(Text(product.sku)),
                                    DataCell(PriceLabel(money: product.price)),
                                    DataCell(_EligibilityCell(
                                        allowed: product.hasEligibility
                                            ? product.eligibility.web
                                            : null)),
                                    DataCell(_EligibilityCell(
                                        allowed: product.hasEligibility
                                            ? product.eligibility.app
                                            : null)),
                                    DataCell(_EligibilityCell(
                                        allowed: product.hasEligibility
                                            ? product.eligibility.lineMiniApp
                                            : null)),
                                  ]);
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

class _EligibilityCell extends StatelessWidget {
  const _EligibilityCell({required this.allowed});

  final bool? allowed;

  @override
  Widget build(BuildContext context) {
    final permitted = allowed;
    final icon = permitted == null
        ? Icons.help_outline
        : permitted
            ? Icons.check_circle_outline
            : Icons.do_not_disturb_on_outlined;
    final color = permitted == true ? GemColors.success : GemColors.inkMuted;
    final label = permitted == null
        ? 'Not provided'
        : permitted
            ? 'Eligible'
            : 'Excluded';
    return Row(
      mainAxisSize: MainAxisSize.min,
      children: [
        Icon(icon, size: 18, color: color),
        const SizedBox(width: GemSpace.xs),
        Text(label),
      ],
    );
  }
}
