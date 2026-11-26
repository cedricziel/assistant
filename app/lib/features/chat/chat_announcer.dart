import 'dart:async';
import 'dart:ui' show FlutterView, PlatformDispatcher, TextDirection;

import 'package:flutter/semantics.dart';
import 'package:flutter_riverpod/flutter_riverpod.dart';

import '../personas/personas_provider.dart';

// -- Lifecycle events --

/// Where the frames that completed a turn came from.
enum TurnSource { live, replay }

/// A fact about the current turn that may deserve a screen-reader
/// announcement. Token frames are deliberately not events.
sealed class ChatLifecycleEvent {
  const ChatLifecycleEvent({required this.at});

  /// When the event happened, from the emitter's injected clock.
  final DateTime at;
}

/// A streaming placeholder was added for a live send. Resets the per-turn
/// stall count and awaiting-input flag.
final class TurnStarted extends ChatLifecycleEvent {
  const TurnStarted({required super.at});
}

/// The reply finished, whether its frames arrived live or by reconnect replay.
final class TurnCompleted extends ChatLifecycleEvent {
  const TurnCompleted({
    required this.content,
    required this.source,
    required super.at,
  });

  /// The full reply in markdown.
  final String content;
  final TurnSource source;
}

/// The turn reconciled to a stopped message.
final class TurnStopped extends ChatLifecycleEvent {
  const TurnStopped({required super.at});
}

/// The turn failed.
final class TurnFailed extends ChatLifecycleEvent {
  const TurnFailed({required this.message, required super.at});

  final String message;
}

/// No status update arrived for the stall threshold.
final class TurnStalled extends ChatLifecycleEvent {
  const TurnStalled({required super.at});
}

/// The turn is waiting on the user (for example a tool approval).
final class TurnAwaitingInput extends ChatLifecycleEvent {
  const TurnAwaitingInput({required this.announcement, required super.at});

  /// Polite text to speak.
  final String announcement;
}

/// The pending input was resolved.
final class TurnInputResolved extends ChatLifecycleEvent {
  const TurnInputResolved({required super.at});
}

// -- Policy --

/// Text to speak and how urgently.
class Announcement {
  const Announcement(this.text, {this.assertive = false});

  final String text;
  final bool assertive;
}

/// Receives announcements that passed the policy's throttle.
typedef AnnouncementSink = void Function(Announcement announcement);

/// Longest excerpt of a reply that is spoken.
const int kAnnouncementExcerptLength = 200;

/// How long a polite announcement may be held while newer ones replace it.
const Duration kAnnouncementHold = Duration(milliseconds: 1500);

/// Maps turn lifecycle events to announcements.
///
/// A polite announcement waits in a single slot. When the slot goes from
/// empty to full its deadline is set to the announcement's time plus
/// [kAnnouncementHold]; a newer polite announcement replaces the held text
/// and keeps the deadline. An assertive announcement is spoken at once and
/// drops the held one.
class ChatAnnouncer {
  ChatAnnouncer({
    required this._personaName,
    required this._sink,
    this._now = DateTime.now,
  });

  final String Function() _personaName;
  final AnnouncementSink _sink;
  final DateTime Function() _now;

  String? _heldText;
  Timer? _timer;
  bool _stallAnnounced = false;
  bool _awaitingInput = false;

  /// Applies [event] to the policy.
  void handle(ChatLifecycleEvent event) {
    switch (event) {
      case TurnStarted():
        _stallAnnounced = false;
        _awaitingInput = false;
        _polite('${_personaName()} is responding', event.at);
      case TurnCompleted(:final content):
        _polite(
          '${_personaName()} replied: ${_excerpt(markdownToPlainText(content))}',
          event.at,
        );
      case TurnStopped():
        _polite('Response stopped', event.at);
      case TurnFailed(:final message):
        _assertive('Response failed: $message');
      case TurnStalled():
        if (_stallAnnounced || _awaitingInput) return;
        _stallAnnounced = true;
        _polite('Still working', event.at);
      case TurnAwaitingInput(:final announcement):
        _awaitingInput = true;
        _polite(announcement, event.at);
      case TurnInputResolved():
        _awaitingInput = false;
    }
  }

  /// Announces [text] through the same throttle, for features outside the
  /// turn lifecycle (code copy, tool approvals).
  void say(String text, {bool assertive = false}) =>
      assertive ? _assertive(text) : _polite(text, _now());

  /// Cancels any held announcement.
  void dispose() {
    _timer?.cancel();
    _timer = null;
    _heldText = null;
  }

  void _polite(String text, DateTime at) {
    final slotWasEmpty = _heldText == null;
    _heldText = text;
    if (!slotWasEmpty) return;
    final wait = at.add(kAnnouncementHold).difference(_now());
    _timer = Timer(wait.isNegative ? Duration.zero : wait, _flush);
  }

  void _assertive(String text) {
    dispose();
    _sink(Announcement(text, assertive: true));
  }

  void _flush() {
    final text = _heldText;
    _timer = null;
    _heldText = null;
    if (text != null) _sink(Announcement(text));
  }
}

String _excerpt(String text) {
  if (text.length <= kAnnouncementExcerptLength) return text;
  final head = text.substring(0, kAnnouncementExcerptLength);
  final atBoundary = text[kAnnouncementExcerptLength] == ' ';
  final lastSpace = head.lastIndexOf(' ');
  final cut = atBoundary || lastSpace < 0 ? head : head.substring(0, lastSpace);
  return '${cut.trimRight()}…';
}

// -- Markdown to plain text --

final _fence = RegExp(r'(```|~~~)[\s\S]*?(?:\1|$)');
final _image = RegExp(r'!\[([^\]]*)\]\([^)]*\)');
final _link = RegExp(r'\[([^\]]*)\]\([^)]*\)');
final _inlineCode = RegExp(r'`([^`]*)`');
final _linePrefix = RegExp(
  r'^[ \t]*(?:#{1,6}[ \t]+|>[ \t]?|(?:[-*+]|\d+[.)])[ \t]+)',
  multiLine: true,
);
final _starEmphasis = RegExp(r'(\*\*|\*|~~)(?=\S)(.+?)(?<=\S)\1');
final _underscoreEmphasis = RegExp(r'(?<!\w)(__|_)(?=\S)(.+?)(?<=\S)\1(?!\w)');
final _whitespace = RegExp(r'\s+');

/// Reduces markdown to the text a listener should hear: emphasis markers
/// dropped, links and images reduced to their text, fenced code replaced by
/// "code block", heading, quote and list markers removed.
String markdownToPlainText(String markdown) {
  return markdown
      .replaceAll(_fence, ' code block ')
      .replaceAllMapped(_image, (m) => m[1]!)
      .replaceAllMapped(_link, (m) => m[1]!)
      .replaceAllMapped(_inlineCode, (m) => m[1]!)
      .replaceAll(_linePrefix, '')
      .replaceAllMapped(_starEmphasis, (m) => m[2]!)
      .replaceAllMapped(_underscoreEmphasis, (m) => m[2]!)
      .replaceAll(_whitespace, ' ')
      .trim();
}

// -- Platform sink --

/// Speaks announcements through the platform accessibility channel.
///
/// Resolves the view per announcement so it also works across multiple
/// windows; with no view available it does nothing.
class ChatAnnouncementSink {
  const ChatAnnouncementSink({FlutterView? Function()? view})
    : _view = view ?? _implicitView;

  final FlutterView? Function() _view;

  static FlutterView? _implicitView() =>
      PlatformDispatcher.instance.implicitView;

  void call(Announcement announcement) {
    final view = _view();
    if (view == null) return;
    unawaited(
      SemanticsService.sendAnnouncement(
        view,
        announcement.text,
        TextDirection.ltr,
        assertiveness: announcement.assertive
            ? Assertiveness.assertive
            : Assertiveness.polite,
      ),
    );
  }
}

/// The sink announcements are sent to. Override in tests to capture them.
final chatAnnouncementSinkProvider = Provider<AnnouncementSink>(
  (ref) => const ChatAnnouncementSink().call,
);

/// The announcer shared by chat and other features, so throttling is global.
final chatAnnouncerProvider = Provider<ChatAnnouncer>((ref) {
  final announcer = ChatAnnouncer(
    personaName: () =>
        ref.read(personasProvider).value?.activePersona?.name ?? 'Assistant',
    sink: ref.watch(chatAnnouncementSinkProvider),
  );
  ref.onDispose(announcer.dispose);
  return announcer;
});
