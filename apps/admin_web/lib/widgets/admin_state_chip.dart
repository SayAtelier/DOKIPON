/*
@cypher schema=1 source_sha256=9bedaedc1e354023f1abc6ff35cb44596aa4f934f99b188005b78bc998981976
MERGE (self:File {path:"apps/admin_web/lib/widgets/admin_state_chip.dart"})
MERGE (chip:Symbol {id:"apps/admin_web/lib/widgets/admin_state_chip.dart::AdminStateChip", kind:"class"})
MERGE (label:Symbol {id:"apps/admin_web/lib/widgets/admin_state_chip.dart::AdminStateChip.labelFor", kind:"method"})
MERGE (color:Symbol {id:"apps/admin_web/lib/widgets/admin_state_chip.dart::AdminStateChip.colorFor", kind:"method"})
MERGE (status:Symbol {id:"packages/domain_models/lib/src/checkout.dart::CheckoutStatus", kind:"enum"})
MERGE (self)-[:DEFINES]->(chip)
MERGE (chip)-[:HAS_METHOD]->(label)
MERGE (chip)-[:HAS_METHOD]->(color)
MERGE (chip)-[:USES_TYPE]->(status)
@endcypher
*/
import 'package:design_system/design_system.dart';
import 'package:domain_models/domain_models.dart';
import 'package:flutter/material.dart';

/// Local state label for order administration; unresolved payments read as review.
class AdminStateChip extends StatelessWidget {
  const AdminStateChip({super.key, required this.status});

  final CheckoutStatus status;

  static String labelFor(CheckoutStatus status) => switch (status) {
        CheckoutStatus.new_ => 'New',
        CheckoutStatus.reserving => 'Reserving',
        CheckoutStatus.stockReserved => 'Stock reserved',
        CheckoutStatus.awaitingPayment => 'Awaiting payment',
        CheckoutStatus.paymentUnknown => 'Needs review',
        CheckoutStatus.confirming => 'Confirming',
        CheckoutStatus.confirmed => 'Confirmed',
        CheckoutStatus.rejected => 'Declined',
        CheckoutStatus.cancelled => 'Cancelled',
        CheckoutStatus.unknown => 'Unknown state',
      };

  static Color colorFor(CheckoutStatus status) => switch (status) {
        CheckoutStatus.paymentUnknown || CheckoutStatus.awaitingPayment =>
          GemColors.warning,
        CheckoutStatus.confirmed => GemColors.success,
        CheckoutStatus.rejected || CheckoutStatus.cancelled => GemColors.danger,
        _ => GemColors.info,
      };

  @override
  Widget build(BuildContext context) {
    final color = colorFor(status);
    return Container(
      padding: const EdgeInsets.symmetric(
        horizontal: GemSpace.sm,
        vertical: GemSpace.xs,
      ),
      decoration: BoxDecoration(
        color: color.withValues(alpha: 0.09),
        border: Border.all(color: color.withValues(alpha: 0.30)),
        borderRadius: const BorderRadius.all(GemRadius.sm),
      ),
      child: Text(
        labelFor(status),
        style: Theme.of(context).textTheme.labelMedium?.copyWith(
              color: color,
              fontWeight: FontWeight.w700,
            ),
      ),
    );
  }
}
