import 'package:flutter/material.dart';

/// Full-screen loading scaffold shown while async providers (e.g.
/// [activeContextProvider]) are still initialising on cold start.
///
/// The router automatically navigates away from this screen once the providers
/// settle — no state management needed here.
class LoadingScreen extends StatelessWidget {
  const LoadingScreen({super.key});

  @override
  Widget build(BuildContext context) {
    return const Scaffold(
      body: Center(
        child: Column(
          mainAxisSize: MainAxisSize.min,
          children: [
            CircularProgressIndicator(),
            SizedBox(height: 16),
            Text('Starting\u2026'),
          ],
        ),
      ),
    );
  }
}
