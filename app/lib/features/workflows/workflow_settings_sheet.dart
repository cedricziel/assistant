import 'package:material_ui/material_ui.dart';

import 'workflow_sheet_handle.dart';

// ---------------------------------------------------------------------------
// Workflow settings sheet

class WorkflowSettingsSheet extends StatefulWidget {
  const WorkflowSettingsSheet({
    super.key,
    required this.nameController,
    required this.descController,
    required this.active,
    required this.maxSteps,
    required this.maxVisitsPerNode,
    required this.onChanged,
  });

  final TextEditingController nameController;
  final TextEditingController descController;
  final bool active;
  final int maxSteps;
  final int maxVisitsPerNode;
  final void Function(bool active, int maxSteps, int maxVisitsPerNode)
  onChanged;

  @override
  State<WorkflowSettingsSheet> createState() => _WorkflowSettingsSheetState();
}

class _WorkflowSettingsSheetState extends State<WorkflowSettingsSheet> {
  late bool _active;
  late int _maxSteps;
  late int _maxVisitsPerNode;

  @override
  void initState() {
    super.initState();
    _active = widget.active;
    _maxSteps = widget.maxSteps;
    _maxVisitsPerNode = widget.maxVisitsPerNode;
  }

  @override
  Widget build(BuildContext context) {
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
            child: Text(
              'Workflow Settings',
              style: Theme.of(context).textTheme.titleMedium,
            ),
          ),
          const SizedBox(height: 12),
          Padding(
            padding: const EdgeInsets.symmetric(horizontal: 16),
            child: Column(
              children: [
                TextField(
                  controller: widget.nameController,
                  decoration: const InputDecoration(
                    labelText: 'Name',
                    border: OutlineInputBorder(),
                  ),
                ),
                const SizedBox(height: 12),
                TextField(
                  controller: widget.descController,
                  maxLines: 2,
                  decoration: const InputDecoration(
                    labelText: 'Description (optional)',
                    border: OutlineInputBorder(),
                    alignLabelWithHint: true,
                  ),
                ),
                const SizedBox(height: 12),
                SwitchListTile(
                  title: const Text('Active'),
                  subtitle: const Text('Enable automatic triggering'),
                  value: _active,
                  onChanged: (v) => setState(() => _active = v),
                  contentPadding: EdgeInsets.zero,
                ),
                Row(
                  children: [
                    Expanded(
                      child: Column(
                        crossAxisAlignment: CrossAxisAlignment.start,
                        children: [
                          Text(
                            'Max steps: $_maxSteps',
                            style: const TextStyle(fontSize: 13),
                          ),
                          Slider(
                            value: _maxSteps.toDouble(),
                            min: 10,
                            max: 500,
                            divisions: 49,
                            onChanged: (v) =>
                                setState(() => _maxSteps = v.round()),
                          ),
                        ],
                      ),
                    ),
                    const SizedBox(width: 12),
                    Expanded(
                      child: Column(
                        crossAxisAlignment: CrossAxisAlignment.start,
                        children: [
                          Text(
                            'Max visits/node: $_maxVisitsPerNode',
                            style: const TextStyle(fontSize: 13),
                          ),
                          Slider(
                            value: _maxVisitsPerNode.toDouble(),
                            min: 1,
                            max: 100,
                            divisions: 99,
                            onChanged: (v) =>
                                setState(() => _maxVisitsPerNode = v.round()),
                          ),
                        ],
                      ),
                    ),
                  ],
                ),
              ],
            ),
          ),
          const SizedBox(height: 12),
          Padding(
            padding: const EdgeInsets.symmetric(horizontal: 16),
            child: FilledButton(
              onPressed: () =>
                  widget.onChanged(_active, _maxSteps, _maxVisitsPerNode),
              child: const Text('Apply Settings'),
            ),
          ),
          const SizedBox(height: 24),
        ],
      ),
    );
  }
}
