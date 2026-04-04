/// A lightweight conversation summary returned by `GET /api/conversations`.
class ConversationSummary {
  const ConversationSummary({
    required this.id,
    required this.title,
    required this.createdAt,
    required this.updatedAt,
  });

  final String id;
  final String title;
  final DateTime createdAt;
  final DateTime updatedAt;

  factory ConversationSummary.fromJson(Map<String, dynamic> json) {
    return ConversationSummary(
      id: json['id'] as String,
      title: json['title'] as String? ?? 'Untitled',
      createdAt: DateTime.parse(json['created_at'] as String),
      updatedAt: DateTime.parse(json['updated_at'] as String),
    );
  }

  ConversationSummary copyWith({
    String? id,
    String? title,
    DateTime? createdAt,
    DateTime? updatedAt,
  }) {
    return ConversationSummary(
      id: id ?? this.id,
      title: title ?? this.title,
      createdAt: createdAt ?? this.createdAt,
      updatedAt: updatedAt ?? this.updatedAt,
    );
  }
}

/// A conversation with its full message history, returned by
/// `GET /api/conversations/{id}`.
class ConversationDetail {
  const ConversationDetail({
    required this.id,
    required this.title,
    required this.createdAt,
    required this.updatedAt,
    required this.messages,
  });

  final String id;
  final String title;
  final DateTime createdAt;
  final DateTime updatedAt;
  final List<Message> messages;

  factory ConversationDetail.fromJson(Map<String, dynamic> json) {
    final msgs = (json['messages'] as List<dynamic>? ?? [])
        .cast<Map<String, dynamic>>()
        .map(Message.fromJson)
        .toList();

    return ConversationDetail(
      id: json['id'] as String,
      title: json['title'] as String? ?? 'Untitled',
      createdAt: DateTime.parse(json['created_at'] as String),
      updatedAt: DateTime.parse(json['updated_at'] as String),
      messages: msgs,
    );
  }
}

/// A single message in a conversation.
class Message {
  const Message({
    required this.id,
    required this.role,
    required this.content,
    required this.turn,
    required this.createdAt,
  });

  final String id;

  /// `"user"` or `"assistant"`.
  final String role;

  final String content;
  final int turn;
  final DateTime createdAt;

  bool get isUser => role == 'user';
  bool get isAssistant => role == 'assistant';

  factory Message.fromJson(Map<String, dynamic> json) {
    return Message(
      id: json['id'] as String,
      role: json['role'] as String,
      content: json['content'] as String? ?? '',
      turn: (json['turn'] as num).toInt(),
      createdAt: DateTime.parse(json['created_at'] as String),
    );
  }
}
