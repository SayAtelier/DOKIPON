/*
@cypher schema=1 source_sha256=99ce02c3e5e92964bd550b52c96aca41bae49c2f33a5f12dac452a96417b68ff
MERGE (self:File {path:"apps/admin_web/lib/widgets/load_failure.dart"})
MERGE (failure:Symbol {id:"apps/admin_web/lib/widgets/load_failure.dart::LoadFailure", kind:"class"})
MERGE (self)-[:DEFINES]->(failure)
@endcypher
*/
import 'package:design_system/design_system.dart';
import 'package:flutter/material.dart';

/// Consistent recoverable load error for an operator screen.
class LoadFailure extends StatelessWidget {
  const LoadFailure({
    super.key,
    required this.message,
    required this.onRetry,
  });

  final String message;
  final VoidCallback onRetry;

  @override
  Widget build(BuildContext context) => Center(
        child: ConstrainedBox(
          constraints: const BoxConstraints(maxWidth: 520),
          child: Column(
            mainAxisSize: MainAxisSize.min,
            crossAxisAlignment: CrossAxisAlignment.stretch,
            children: [
              StatusBanner(message: message, tone: BannerTone.danger),
              const SizedBox(height: GemSpace.md),
              Align(
                alignment: Alignment.center,
                child: FilledButton.icon(
                  onPressed: onRetry,
                  icon: const Icon(Icons.refresh),
                  label: const Text('Retry'),
                ),
              ),
            ],
          ),
        ),
      );
}
