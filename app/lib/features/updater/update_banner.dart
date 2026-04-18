import 'package:dio/dio.dart';
import 'package:flutter/material.dart';
import 'package:flutter_riverpod/flutter_riverpod.dart';

import 'artifact_downloader.dart';
import 'update_provider.dart';
import 'update_service.dart';

/// Wraps [child] and shows a dismissible update-available banner above it
/// whenever [updateCheckProvider] returns a non-null [UpdateInfo].
class UpdateBannerWrapper extends ConsumerWidget {
  const UpdateBannerWrapper({super.key, required this.child});

  final Widget child;

  @override
  Widget build(BuildContext context, WidgetRef ref) {
    final updateAsync = ref.watch(updateCheckProvider);

    return updateAsync.when(
      loading: () => child,
      error: (_, _) => child,
      data: (info) {
        if (info == null) return child;
        return _WithBanner(info: info, child: child);
      },
    );
  }
}

class _WithBanner extends ConsumerWidget {
  const _WithBanner({required this.info, required this.child});

  final UpdateInfo info;
  final Widget child;

  @override
  Widget build(BuildContext context, WidgetRef ref) {
    final colorScheme = Theme.of(context).colorScheme;

    return Column(
      children: [
        Material(
          color: colorScheme.primaryContainer,
          child: Padding(
            padding: const EdgeInsets.symmetric(horizontal: 16, vertical: 10),
            child: Row(
              children: [
                Icon(
                  Icons.system_update_alt,
                  size: 20,
                  color: colorScheme.onPrimaryContainer,
                ),
                const SizedBox(width: 10),
                Expanded(
                  child: Column(
                    crossAxisAlignment: CrossAxisAlignment.start,
                    children: [
                      Text(
                        'Update available — ${info.latestVersion}',
                        style: TextStyle(
                          fontWeight: FontWeight.w600,
                          color: colorScheme.onPrimaryContainer,
                        ),
                      ),
                      Text(
                        info.shortReleaseNotes,
                        style: TextStyle(
                          fontSize: 12,
                          color: colorScheme.onPrimaryContainer.withValues(
                            alpha: 0.8,
                          ),
                        ),
                        maxLines: 1,
                        overflow: TextOverflow.ellipsis,
                      ),
                    ],
                  ),
                ),
                const SizedBox(width: 8),
                FilledButton.tonal(
                  onPressed: () => _onDownload(context, ref),
                  child: const Text('Download'),
                ),
                const SizedBox(width: 4),
                IconButton(
                  icon: const Icon(Icons.close),
                  color: colorScheme.onPrimaryContainer,
                  tooltip: 'Dismiss',
                  onPressed: () =>
                      ref.read(updateCheckProvider.notifier).dismiss(),
                ),
              ],
            ),
          ),
        ),
        Expanded(child: child),
      ],
    );
  }

  void _onDownload(BuildContext context, WidgetRef ref) {
    showDialog<void>(
      context: context,
      builder: (_) => _DownloadDialog(info: info),
    );
  }
}

/// Dialog shown when the user taps "Download".
/// The actual download pipeline (ArtifactDownloader → InstallerLauncher)
/// is initiated from here.
class _DownloadDialog extends StatelessWidget {
  const _DownloadDialog({required this.info});

  final UpdateInfo info;

  @override
  Widget build(BuildContext context) {
    return AlertDialog(
      title: const Text('Download update'),
      content: Text(
        'Download version ${info.latestVersion} and install it?\n\n'
        'The app will restart after the update is applied.',
      ),
      actions: [
        TextButton(
          onPressed: () => Navigator.of(context).pop(),
          child: const Text('Cancel'),
        ),
        FilledButton(
          onPressed: () {
            Navigator.of(context).pop();
            _startDownload(context);
          },
          child: const Text('Download & Install'),
        ),
      ],
    );
  }

  void _startDownload(BuildContext context) {
    showDialog<void>(
      context: context,
      barrierDismissible: false,
      builder: (_) => _ProgressDialog(info: info),
    );
  }
}

/// Shows download progress and drives the
/// ArtifactDownloader → ChecksumVerifier → InstallerLauncher pipeline.
class _ProgressDialog extends StatefulWidget {
  const _ProgressDialog({required this.info});

  final UpdateInfo info;

  @override
  State<_ProgressDialog> createState() => _ProgressDialogState();
}

class _ProgressDialogState extends State<_ProgressDialog> {
  double? _progress;
  String _status = 'Starting download…';
  final CancelToken _cancelToken = CancelToken();

  @override
  void initState() {
    super.initState();
    _run();
  }

  @override
  void dispose() {
    // Cancel any in-flight download when the dialog is removed from the tree.
    if (!_cancelToken.isCancelled) {
      _cancelToken.cancel();
    }
    super.dispose();
  }

  Future<void> _run() async {
    try {
      final downloader = ArtifactDownloader();
      await downloader.downloadAndInstall(
        widget.info.release,
        onProgress: (received, total) {
          if (!mounted) return;
          setState(() {
            _progress = total > 0 ? received / total : null;
            _status = total > 0
                ? 'Downloading… ${(received / 1024 / 1024).toStringAsFixed(1)} MB'
                : 'Downloading…';
          });
        },
        onStatus: (msg) {
          if (!mounted) return;
          setState(() => _status = msg);
        },
        cancelToken: _cancelToken,
      );
    } on DioException catch (e) {
      if (e.type == DioExceptionType.cancel) return; // User cancelled — silent.
      if (!mounted) return;
      Navigator.of(context).pop();
      ScaffoldMessenger.of(
        context,
      ).showSnackBar(SnackBar(content: Text('Download failed: ${e.message}')));
    } catch (e) {
      if (!mounted) return;
      Navigator.of(context).pop();
      ScaffoldMessenger.of(
        context,
      ).showSnackBar(SnackBar(content: Text('Update failed: $e')));
    }
  }

  @override
  Widget build(BuildContext context) {
    return AlertDialog(
      title: const Text('Downloading update'),
      content: Column(
        mainAxisSize: MainAxisSize.min,
        children: [
          LinearProgressIndicator(value: _progress),
          const SizedBox(height: 12),
          Text(_status),
        ],
      ),
      actions: [
        TextButton(
          onPressed: () {
            _cancelToken.cancel();
            Navigator.of(context).pop();
          },
          child: const Text('Cancel'),
        ),
      ],
    );
  }
}
