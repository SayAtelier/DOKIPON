/*
@cypher schema=1 source_sha256=c1580bb531f69be8df692496cb9e05e51532364611d12f28205324eb6c59c4d6
MERGE (self:File {path:"apps/admin_web/lib/screens/login_screen.dart"})
MERGE (screen:Symbol {id:"apps/admin_web/lib/screens/login_screen.dart::LoginScreen", kind:"class"})
MERGE (state:Symbol {id:"apps/admin_web/lib/screens/login_screen.dart::_LoginScreenState", kind:"class"})
MERGE (submit:Symbol {id:"apps/admin_web/lib/screens/login_screen.dart::_LoginScreenState._submit", kind:"method"})
MERGE (controller:Symbol {id:"packages/storefront_state/lib/src/admin_console_controller.dart::AdminConsoleController", kind:"class"})
MERGE (self)-[:DEFINES]->(screen)
MERGE (self)-[:DEFINES]->(state)
MERGE (screen)-[:USES_TYPE]->(controller)
MERGE (state)-[:HAS_METHOD]->(submit)
MERGE (submit)-[:CALLS]->(controller)
@endcypher
*/
import 'package:design_system/design_system.dart';
import 'package:flutter/material.dart';
import 'package:storefront_state/storefront_state.dart';

class LoginScreen extends StatefulWidget {
  const LoginScreen({super.key, required this.controller});

  final AdminConsoleController controller;

  @override
  State<LoginScreen> createState() => _LoginScreenState();
}

class _LoginScreenState extends State<LoginScreen> {
  final _formKey = GlobalKey<FormState>();
  final _username = TextEditingController();
  final _password = TextEditingController();

  Future<void> _submit() async {
    if (!_formKey.currentState!.validate()) return;
    await widget.controller.signIn(
      username: _username.text.trim(),
      password: _password.text,
    );
  }

  @override
  void dispose() {
    _username.dispose();
    _password.dispose();
    super.dispose();
  }

  @override
  Widget build(BuildContext context) => Scaffold(
        body: Center(
          child: SingleChildScrollView(
            padding: const EdgeInsets.all(GemSpace.lg),
            child: ConstrainedBox(
              constraints: const BoxConstraints(maxWidth: 440),
              child: Card(
                child: Padding(
                  padding: const EdgeInsets.all(GemSpace.xl),
                  child: AnimatedBuilder(
                    animation: widget.controller,
                    builder: (context, _) => Form(
                      key: _formKey,
                      child: Column(
                        crossAxisAlignment: CrossAxisAlignment.stretch,
                        children: [
                          const Icon(Icons.diamond_outlined,
                              color: GemColors.brand, size: 42),
                          const SizedBox(height: GemSpace.md),
                          Text(
                            'Commerce administration',
                            textAlign: TextAlign.center,
                            style: Theme.of(context).textTheme.headlineSmall,
                          ),
                          const SizedBox(height: GemSpace.sm),
                          Text(
                            'Sign in with your operator account.',
                            textAlign: TextAlign.center,
                            style: Theme.of(context).textTheme.bodyMedium?.copyWith(
                                  color: GemColors.inkMuted,
                                ),
                          ),
                          const SizedBox(height: GemSpace.lg),
                          if (widget.controller.sessionNotice case final notice?) ...[
                            StatusBanner(
                              message: notice,
                              tone: BannerTone.warning,
                            ),
                            const SizedBox(height: GemSpace.md),
                          ],
                          if (widget.controller.loginError case final error?) ...[
                            StatusBanner(
                              message: error,
                              tone: BannerTone.danger,
                            ),
                            const SizedBox(height: GemSpace.md),
                          ],
                          TextFormField(
                            controller: _username,
                            autofillHints: const [AutofillHints.username],
                            textInputAction: TextInputAction.next,
                            decoration: const InputDecoration(
                              labelText: 'Username',
                              prefixIcon: Icon(Icons.person_outline),
                            ),
                            validator: (value) => value == null || value.trim().isEmpty
                                ? 'Enter your username.'
                                : null,
                          ),
                          const SizedBox(height: GemSpace.md),
                          TextFormField(
                            controller: _password,
                            obscureText: true,
                            autofillHints: const [AutofillHints.password],
                            onFieldSubmitted: (_) => _submit(),
                            decoration: const InputDecoration(
                              labelText: 'Password',
                              prefixIcon: Icon(Icons.lock_outline),
                            ),
                            validator: (value) => value == null || value.isEmpty
                                ? 'Enter your password.'
                                : null,
                          ),
                          const SizedBox(height: GemSpace.lg),
                          FilledButton.icon(
                            onPressed: widget.controller.isSigningIn
                                ? null
                                : _submit,
                            icon: widget.controller.isSigningIn
                                ? const SizedBox.square(
                                    dimension: 18,
                                    child: CircularProgressIndicator(strokeWidth: 2),
                                  )
                                : const Icon(Icons.login),
                            label: Text(widget.controller.isSigningIn
                                ? 'Signing in…'
                                : 'Sign in'),
                          ),
                        ],
                      ),
                    ),
                  ),
                ),
              ),
            ),
          ),
        ),
      );
}
