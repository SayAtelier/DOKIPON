/*
@cypher schema=1 source_sha256=022c74c2d535a16230c8d2766ff1cd137722a1e152f96b913d4e253dbe781930
MERGE (self:File {path:"apps/admin_web/lib/admin_console_app.dart"})
MERGE (app:Symbol {id:"apps/admin_web/lib/admin_console_app.dart::AdminConsoleApp", kind:"class"})
MERGE (root:Symbol {id:"apps/admin_web/lib/admin_console_app.dart::AdminConsoleRoot", kind:"class"})
MERGE (state:Symbol {id:"apps/admin_web/lib/admin_console_app.dart::_AdminConsoleRootState", kind:"class"})
MERGE (shell:Symbol {id:"apps/admin_web/lib/admin_console_app.dart::AdminConsoleShell", kind:"class"})
MERGE (controller:Symbol {id:"packages/storefront_state/lib/src/admin_console_controller.dart::AdminConsoleController", kind:"class"})
MERGE (login:Symbol {id:"apps/admin_web/lib/screens/login_screen.dart::LoginScreen", kind:"class"})
MERGE (orders:Symbol {id:"apps/admin_web/lib/screens/orders_screen.dart::OrdersScreen", kind:"class"})
MERGE (detail:Symbol {id:"apps/admin_web/lib/screens/order_detail_screen.dart::OrderDetailScreen", kind:"class"})
MERGE (catalog:Symbol {id:"apps/admin_web/lib/screens/catalog_screen.dart::CatalogScreen", kind:"class"})
MERGE (self)-[:DEFINES]->(app)
MERGE (self)-[:DEFINES]->(root)
MERGE (self)-[:DEFINES]->(state)
MERGE (self)-[:DEFINES]->(shell)
MERGE (app)-[:USES_TYPE]->(controller)
MERGE (state)-[:USES_TYPE]->(controller)
MERGE (state)-[:CREATES]->(login)
MERGE (state)-[:CREATES]->(orders)
MERGE (state)-[:CREATES]->(detail)
MERGE (state)-[:CREATES]->(catalog)
@endcypher
*/
import 'dart:async';

// The shared design_system also exports a storefront `CatalogScreen`. Prefix
// these imports so the admin screens are unambiguous at every use site; a bare
// `CatalogScreen` would resolve to whichever library the analyzer sees first.
import 'package:design_system/design_system.dart';
import 'package:flutter/material.dart';
import 'package:storefront_state/storefront_state.dart';

import 'screens/catalog_screen.dart' as admin_screens;
import 'screens/login_screen.dart';
import 'screens/order_detail_screen.dart';
import 'screens/orders_screen.dart';

class AdminConsoleApp extends StatelessWidget {
  const AdminConsoleApp({super.key, required this.controller});

  final AdminConsoleController controller;

  @override
  Widget build(BuildContext context) => MaterialApp(
        title: 'GemArc Admin',
        theme: GemTheme.light(),
        home: AdminConsoleRoot(controller: controller),
      );
}

enum AdminSection { orders, orderDetail, catalog }

class AdminConsoleRoot extends StatefulWidget {
  const AdminConsoleRoot({super.key, required this.controller});

  final AdminConsoleController controller;

  @override
  State<AdminConsoleRoot> createState() => _AdminConsoleRootState();
}

class _AdminConsoleRootState extends State<AdminConsoleRoot> {
  AdminSection _section = AdminSection.orders;

  Future<void> _openOrder(String orderId) async {
    setState(() => _section = AdminSection.orderDetail);
    await widget.controller.loadOrderDetail(orderId);
  }

  Widget _screen() => switch (_section) {
        AdminSection.orders => OrdersScreen(
            controller: widget.controller,
            onOpenOrder: (id) => unawaited(_openOrder(id)),
          ),
        AdminSection.orderDetail => OrderDetailScreen(
            controller: widget.controller,
            onBack: () => setState(() => _section = AdminSection.orders),
          ),
        AdminSection.catalog =>
          admin_screens.CatalogScreen(controller: widget.controller),
      };

  @override
  Widget build(BuildContext context) => AnimatedBuilder(
        animation: widget.controller,
        builder: (context, _) {
          if (!widget.controller.isAuthenticated) {
            return LoginScreen(controller: widget.controller);
          }
          return AdminConsoleShell(
            section: _section,
            onSectionChanged: (section) => setState(() => _section = section),
            onSignOut: widget.controller.signOut,
            child: _screen(),
          );
        },
      );
}

class AdminConsoleShell extends StatelessWidget {
  const AdminConsoleShell({
    super.key,
    required this.section,
    required this.onSectionChanged,
    required this.onSignOut,
    required this.child,
  });

  final AdminSection section;
  final ValueChanged<AdminSection> onSectionChanged;
  final VoidCallback onSignOut;
  final Widget child;

  String get _title => switch (section) {
        AdminSection.orders => 'Orders',
        AdminSection.orderDetail => 'Order detail',
        AdminSection.catalog => 'Catalog',
      };

  @override
  Widget build(BuildContext context) {
    final compact = MediaQuery.sizeOf(context).width < 920;
    final selectedIndex = section == AdminSection.catalog ? 1 : 0;
    return Scaffold(
      appBar: AppBar(
        title: Row(
          children: [
            const Icon(Icons.diamond_outlined, color: GemColors.brand),
            const SizedBox(width: GemSpace.sm),
            Text('GemArc  /  $_title'),
          ],
        ),
        actions: [
          if (section == AdminSection.orderDetail)
            TextButton.icon(
              onPressed: () => onSectionChanged(AdminSection.orders),
              icon: const Icon(Icons.arrow_back),
              label: const Text('Orders'),
            ),
          IconButton(
            tooltip: 'Sign out',
            onPressed: onSignOut,
            icon: const Icon(Icons.logout),
          ),
          const SizedBox(width: GemSpace.sm),
        ],
      ),
      body: Row(
        children: [
          if (!compact)
            NavigationRail(
              selectedIndex: selectedIndex,
              onDestinationSelected: (index) => onSectionChanged(
                index == 0 ? AdminSection.orders : AdminSection.catalog,
              ),
              labelType: NavigationRailLabelType.all,
              destinations: const [
                NavigationRailDestination(
                  icon: Icon(Icons.receipt_long_outlined),
                  selectedIcon: Icon(Icons.receipt_long),
                  label: Text('Orders'),
                ),
                NavigationRailDestination(
                  icon: Icon(Icons.inventory_2_outlined),
                  selectedIcon: Icon(Icons.inventory_2),
                  label: Text('Catalog'),
                ),
              ],
            ),
          Expanded(child: child),
        ],
      ),
      bottomNavigationBar: compact
          ? NavigationBar(
              selectedIndex: selectedIndex,
              onDestinationSelected: (index) => onSectionChanged(
                index == 0 ? AdminSection.orders : AdminSection.catalog,
              ),
              destinations: const [
                NavigationDestination(
                  icon: Icon(Icons.receipt_long_outlined),
                  selectedIcon: Icon(Icons.receipt_long),
                  label: 'Orders',
                ),
                NavigationDestination(
                  icon: Icon(Icons.inventory_2_outlined),
                  selectedIcon: Icon(Icons.inventory_2),
                  label: 'Catalog',
                ),
              ],
            )
          : null,
    );
  }
}
