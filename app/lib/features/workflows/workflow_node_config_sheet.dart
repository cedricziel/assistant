import 'package:material_ui/material_ui.dart';

import 'workflow_editor_model.dart';
import 'workflow_sheet_handle.dart';

// ---------------------------------------------------------------------------
// Node config bottom sheet

class NodeConfigSheet extends StatefulWidget {
  const NodeConfigSheet({
    super.key,
    required this.node,
    this.personaNames = const [],
  });

  final EditorNode node;

  /// List of (id, name) pairs for persona selection on assistant_turn nodes.
  final List<(String, String)> personaNames;

  @override
  State<NodeConfigSheet> createState() => _NodeConfigSheetState();
}

class _NodeConfigSheetState extends State<NodeConfigSheet> {
  late Map<String, dynamic> _config;
  late TextEditingController _cronCtrl;
  late TextEditingController _eventCtrl;
  late TextEditingController _promptCtrl;
  late TextEditingController _urlCtrl;
  late TextEditingController _keyCtrl;
  String _method = 'GET';
  String? _personaId;
  bool _disabled = false;

  @override
  void initState() {
    super.initState();
    _config = Map<String, dynamic>.from(widget.node.config);
    _cronCtrl = TextEditingController(text: _config['cron'] as String? ?? '');
    _eventCtrl = TextEditingController(text: _config['event'] as String? ?? '');
    _promptCtrl = TextEditingController(
      text: _config['prompt'] as String? ?? '',
    );
    _urlCtrl = TextEditingController(text: _config['url'] as String? ?? '');
    _keyCtrl = TextEditingController(text: _config['key'] as String? ?? '');
    _method = _config['method'] as String? ?? 'GET';
    _personaId = _config['persona_id'] as String?;
    _disabled = _config['disabled'] == true;
  }

  @override
  void dispose() {
    _cronCtrl.dispose();
    _eventCtrl.dispose();
    _promptCtrl.dispose();
    _urlCtrl.dispose();
    _keyCtrl.dispose();
    super.dispose();
  }

  void _save() {
    final nodeType = _config['type'] as String? ?? '';
    switch (nodeType) {
      case 'schedule':
        _config['cron'] = _cronCtrl.text.trim();
        break;
      case 'event':
        _config['event'] = _eventCtrl.text.trim();
        break;
      case 'assistant_turn':
        _config['prompt'] = _promptCtrl.text;
        if (_personaId != null) {
          _config['persona_id'] = _personaId;
        } else {
          _config.remove('persona_id');
        }
        break;
      case 'http_request':
        _config['url'] = _urlCtrl.text.trim();
        _config['method'] = _method;
        break;
      case 'has_trigger_payload_key':
        _config['key'] = _keyCtrl.text.trim();
        break;
    }

    if (_disabled) {
      _config['disabled'] = true;
    } else {
      _config.remove('disabled');
    }

    final updated = widget.node.copyWith(config: _config);
    Navigator.of(context).pop(updated);
  }

  @override
  Widget build(BuildContext context) {
    final nodeType = _config['type'] as String? ?? widget.node.kind;
    final color = widget.node.color;

    return Padding(
      padding: EdgeInsets.only(
        bottom: MediaQuery.of(context).viewInsets.bottom,
      ),
      child: Column(
        mainAxisSize: MainAxisSize.min,
        crossAxisAlignment: CrossAxisAlignment.stretch,
        children: [
          const SheetHandle(),
          Padding(
            padding: const EdgeInsets.fromLTRB(16, 8, 16, 0),
            child: Row(
              children: [
                Icon(widget.node.icon, color: color, size: 20),
                const SizedBox(width: 8),
                Text(
                  'Configure: ${widget.node.label}',
                  style: Theme.of(context).textTheme.titleMedium,
                ),
              ],
            ),
          ),
          const SizedBox(height: 4),
          SwitchListTile.adaptive(
            title: const Text('Enabled'),
            subtitle: _disabled
                ? const Text('This node will be skipped during execution')
                : null,
            value: !_disabled,
            onChanged: (v) => setState(() => _disabled = !v),
            contentPadding: const EdgeInsets.symmetric(horizontal: 16),
          ),
          const SizedBox(height: 8),
          Padding(
            padding: const EdgeInsets.symmetric(horizontal: 16),
            child: _buildConfigFields(nodeType),
          ),
          const SizedBox(height: 16),
          Padding(
            padding: const EdgeInsets.symmetric(horizontal: 16),
            child: FilledButton(onPressed: _save, child: const Text('Apply')),
          ),
          const SizedBox(height: 24),
        ],
      ),
    );
  }

  Widget _buildConfigFields(String nodeType) {
    switch (nodeType) {
      case 'manual':
      case 'webhook':
      case 'always_true':
      case 'always_false':
        return Container(
          padding: const EdgeInsets.all(12),
          decoration: BoxDecoration(
            color: Theme.of(context).colorScheme.onSurface
                .withValues(alpha: 0.04),
            borderRadius: BorderRadius.circular(8),
          ),
          child: Text(
            'No configuration required for this node type.',
            style: TextStyle(
              fontSize: 13,
              color: Theme.of(context).colorScheme.onSurfaceVariant,
            ),
          ),
        );

      case 'schedule':
        return TextField(
          controller: _cronCtrl,
          decoration: const InputDecoration(
            labelText: 'Cron expression',
            hintText: '0 * * * *',
            helperText: 'Standard 5-field cron: min hour day month weekday',
            border: OutlineInputBorder(),
          ),
          style: const TextStyle(fontFamily: 'monospace'),
        );

      case 'event':
        return TextField(
          controller: _eventCtrl,
          decoration: const InputDecoration(
            labelText: 'Event topic',
            hintText: 'message.completed',
            border: OutlineInputBorder(),
          ),
          style: const TextStyle(fontFamily: 'monospace'),
        );

      case 'assistant_turn':
        return Column(
          children: [
            if (widget.personaNames.isNotEmpty) ...[
              DropdownButtonFormField<String?>(
                initialValue: _personaId,
                decoration: const InputDecoration(
                  labelText: 'Persona',
                  border: OutlineInputBorder(),
                ),
                items: [
                  const DropdownMenuItem<String?>(
                    value: null,
                    child: Text('Default'),
                  ),
                  ...widget.personaNames.map(
                    (p) => DropdownMenuItem<String?>(
                      value: p.$1,
                      child: Text(p.$2),
                    ),
                  ),
                ],
                onChanged: (v) => setState(() => _personaId = v),
              ),
              const SizedBox(height: 12),
            ],
            TextField(
              controller: _promptCtrl,
              minLines: 4,
              maxLines: null,
              decoration: const InputDecoration(
                labelText: 'Prompt',
                hintText: 'Enter the assistant prompt…',
                border: OutlineInputBorder(),
                alignLabelWithHint: true,
              ),
            ),
          ],
        );

      case 'http_request':
        return Column(
          children: [
            DropdownButtonFormField<String>(
              initialValue: _method,
              decoration: const InputDecoration(
                labelText: 'HTTP method',
                border: OutlineInputBorder(),
              ),
              items: [
                'GET',
                'POST',
                'PUT',
                'PATCH',
                'DELETE',
              ].map((m) => DropdownMenuItem(value: m, child: Text(m))).toList(),
              onChanged: (v) => setState(() => _method = v ?? 'GET'),
            ),
            const SizedBox(height: 12),
            TextField(
              controller: _urlCtrl,
              decoration: const InputDecoration(
                labelText: 'URL',
                hintText: 'https://example.com/webhook',
                border: OutlineInputBorder(),
              ),
              keyboardType: TextInputType.url,
            ),
          ],
        );

      case 'has_trigger_payload_key':
        return TextField(
          controller: _keyCtrl,
          decoration: const InputDecoration(
            labelText: 'Payload key',
            hintText: 'e.g. user_id',
            border: OutlineInputBorder(),
          ),
          style: const TextStyle(fontFamily: 'monospace'),
        );

      default:
        return Text('Unknown node type: $nodeType');
    }
  }
}
