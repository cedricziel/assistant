import 'package:material_ui/material_ui.dart';

import 'workflow_sheet_handle.dart';

// ---------------------------------------------------------------------------
// Node palette bottom sheet

class NodePalette extends StatelessWidget {
  const NodePalette({super.key, required this.onSelected});

  final void Function(String kind, Map<String, dynamic> config) onSelected;

  @override
  Widget build(BuildContext context) {
    return DraggableScrollableSheet(
      initialChildSize: 0.6,
      maxChildSize: 0.9,
      minChildSize: 0.4,
      expand: false,
      builder: (ctx, sc) => Column(
        children: [
          const SheetHandle(),
          Padding(
            padding: const EdgeInsets.fromLTRB(16, 8, 16, 4),
            child: Row(
              children: [
                Text(
                  'Add Node',
                  style: Theme.of(context).textTheme.titleMedium,
                ),
                const Spacer(),
                IconButton(
                  icon: const Icon(Icons.close),
                  onPressed: () => Navigator.of(ctx).pop(),
                ),
              ],
            ),
          ),
          Expanded(
            child: ListView(
              controller: sc,
              padding: const EdgeInsets.symmetric(horizontal: 16),
              children: [
                _PaletteSection(
                  label: 'Triggers',
                  color: Colors.blue,
                  items: [
                    _PaletteItem(
                      title: 'Manual',
                      subtitle: 'Triggered manually or by test run',
                      icon: Icons.touch_app,
                      onTap: () => onSelected('trigger', {'type': 'manual'}),
                    ),
                    _PaletteItem(
                      title: 'Webhook',
                      subtitle: 'Triggered via inbound HTTP webhook',
                      icon: Icons.webhook,
                      onTap: () => onSelected('trigger', {'type': 'webhook'}),
                    ),
                    _PaletteItem(
                      title: 'Schedule',
                      subtitle: 'Triggered on a cron schedule',
                      icon: Icons.schedule,
                      onTap: () => onSelected('trigger', {
                        'type': 'schedule',
                        'cron': '0 * * * *',
                      }),
                    ),
                    _PaletteItem(
                      title: 'Event',
                      subtitle: 'Triggered by an internal bus event',
                      icon: Icons.notifications_active_outlined,
                      onTap: () => onSelected('trigger', {
                        'type': 'event',
                        'event': 'message.completed',
                      }),
                    ),
                  ],
                ),
                const SizedBox(height: 8),
                _PaletteSection(
                  label: 'Actions',
                  color: Colors.green,
                  items: [
                    _PaletteItem(
                      title: 'Assistant Turn',
                      subtitle: 'Run a prompt through the assistant',
                      icon: Icons.smart_toy_outlined,
                      onTap: () => onSelected('action', {
                        'type': 'assistant_turn',
                        'prompt': '',
                      }),
                    ),
                    _PaletteItem(
                      title: 'HTTP Request',
                      subtitle: 'Make an outbound HTTP call',
                      icon: Icons.http,
                      onTap: () => onSelected('action', {
                        'type': 'http_request',
                        'url': 'https://',
                        'method': 'GET',
                      }),
                    ),
                  ],
                ),
                const SizedBox(height: 8),
                _PaletteSection(
                  label: 'Conditions',
                  color: Colors.orange,
                  items: [
                    _PaletteItem(
                      title: 'Always True',
                      subtitle: 'Always routes to the "true" branch',
                      icon: Icons.check_circle_outline,
                      onTap: () =>
                          onSelected('condition', {'type': 'always_true'}),
                    ),
                    _PaletteItem(
                      title: 'Always False',
                      subtitle: 'Always routes to the "false" branch',
                      icon: Icons.cancel_outlined,
                      onTap: () =>
                          onSelected('condition', {'type': 'always_false'}),
                    ),
                    _PaletteItem(
                      title: 'Payload Key Check',
                      subtitle: 'Check if trigger payload has a key',
                      icon: Icons.vpn_key_outlined,
                      onTap: () => onSelected('condition', {
                        'type': 'has_trigger_payload_key',
                        'key': '',
                      }),
                    ),
                  ],
                ),
                const SizedBox(height: 24),
              ],
            ),
          ),
        ],
      ),
    );
  }
}

class _PaletteSection extends StatelessWidget {
  const _PaletteSection({
    required this.label,
    required this.color,
    required this.items,
  });

  final String label;
  final Color color;
  final List<Widget> items;

  @override
  Widget build(BuildContext context) {
    return Column(
      crossAxisAlignment: CrossAxisAlignment.start,
      children: [
        Row(
          children: [
            Container(
              width: 12,
              height: 12,
              decoration: BoxDecoration(color: color, shape: BoxShape.circle),
            ),
            const SizedBox(width: 6),
            Text(
              label.toUpperCase(),
              style: TextStyle(
                fontSize: 11,
                fontWeight: FontWeight.w700,
                letterSpacing: 0.5,
                color: color,
              ),
            ),
          ],
        ),
        const SizedBox(height: 6),
        ...items,
      ],
    );
  }
}

class _PaletteItem extends StatelessWidget {
  const _PaletteItem({
    required this.title,
    required this.subtitle,
    required this.icon,
    required this.onTap,
  });

  final String title;
  final String subtitle;
  final IconData icon;
  final VoidCallback onTap;

  @override
  Widget build(BuildContext context) {
    return ListTile(
      leading: Icon(icon, size: 20),
      title: Text(title, style: const TextStyle(fontWeight: FontWeight.w600)),
      subtitle: Text(subtitle, style: const TextStyle(fontSize: 11)),
      onTap: onTap,
      shape: RoundedRectangleBorder(borderRadius: BorderRadius.circular(8)),
    );
  }
}
