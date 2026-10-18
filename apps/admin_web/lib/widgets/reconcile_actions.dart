/*
@cypher schema=1 source_sha256=aba20dcbe86205a23313099d709a6fc8dd53abbd3dcb697da8312ebaf267e678
MERGE (self:File {path:"apps/admin_web/lib/widgets/reconcile_actions.dart"})
MERGE (actions:Symbol {id:"apps/admin_web/lib/widgets/reconcile_actions.dart::ReconcileActions", kind:"class"})
MERGE (build:Symbol {id:"apps/admin_web/lib/widgets/reconcile_actions.dart::ReconcileActions.build", kind:"method"})
MERGE (status:Symbol {id:"packages/domain_models/lib/src/checkout.dart::CheckoutStatus", kind:"enum"})
MERGE (self)-[:DEFINES]->(actions)
MERGE (actions)-[:HAS_METHOD]->(build)
MERGE (actions)-[:USES_TYPE]->(status)
@endcypher
*/
import 'package:design_system/design_system.dart';
import 'package:domain_models/domain_models.dart';
import 'package:flutter/material.dart';

/// Payment reconciliation is actionable only while the provider verdict is unknown.
class ReconcileActions extends StatelessWidget {
  const ReconcileActions({
    super.key,
    required this.status,
    required this.isWorking,
    required this.onMarkAuthorized,
    required this.onMarkDeclined,
  });

  final CheckoutStatus status;
  final bool isWorking;
  final VoidCallback onMarkAuthorized;
  final VoidCallback onMarkDeclined;

  @override
  Widget build(BuildContext context) {
    if (status != CheckoutStatus.paymentUnknown) return const SizedBox.shrink();
    return Card(
      child: Padding(
        padding: const EdgeInsets.all(GemSpace.md),
        child: Column(
          crossAxisAlignment: CrossAxisAlignment.start,
          children: [
            Text('Payment needs review',
                style: Theme.of(context).textTheme.titleMedium),
            const SizedBox(height: GemSpace.xs),
            const Text(
              'Confirm the provider outcome before recording the order state.',
            ),
            const SizedBox(height: GemSpace.md),
            Wrap(
              spacing: GemSpace.sm,
              runSpacing: GemSpace.sm,
              children: [
                FilledButton.icon(
                  onPressed: isWorking ? null : onMarkAuthorized,
                  icon: const Icon(Icons.check_circle_outline),
                  label: const Text('Mark authorized'),
                ),
                OutlinedButton.icon(
                  onPressed: isWorking ? null : onMarkDeclined,
                  icon: const Icon(Icons.block_outlined),
                  label: const Text('Mark declined'),
                ),
                if (isWorking)
                  const Padding(
                    padding: EdgeInsets.all(GemSpace.md),
                    child: SizedBox.square(
                      dimension: 20,
                      child: CircularProgressIndicator(strokeWidth: 2),
                    ),
                  ),
              ],
            ),
          ],
        ),
      ),
    );
  }
}
