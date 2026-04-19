import XCTest
@testable import AssistantIntents

final class APIClientTests: XCTestCase {

    // MARK: - JSON Decoding Tests

    func testQuickMessageResponseDecoding() throws {
        let json = """
        {
            "conversation_id": "abc-123",
            "message_id": "msg-456",
            "answer": "Hello, world!"
        }
        """.data(using: .utf8)!

        let response = try JSONDecoder().decode(
            AssistantAPIClient.QuickMessageResponse.self,
            from: json
        )

        XCTAssertEqual(response.conversationId, "abc-123")
        XCTAssertEqual(response.messageId, "msg-456")
        XCTAssertEqual(response.answer, "Hello, world!")
    }

    func testQuickMessageResponseDecodingWithoutMessageId() throws {
        let json = """
        {
            "conversation_id": "abc-123",
            "answer": "Hello!"
        }
        """.data(using: .utf8)!

        let response = try JSONDecoder().decode(
            AssistantAPIClient.QuickMessageResponse.self,
            from: json
        )

        XCTAssertEqual(response.conversationId, "abc-123")
        XCTAssertNil(response.messageId)
        XCTAssertEqual(response.answer, "Hello!")
    }

    func testPersonaSummaryDecoding() throws {
        let json = """
        [
            {"id": "default", "name": "Default", "description": "The default persona"}
        ]
        """.data(using: .utf8)!

        let personas = try JSONDecoder().decode(
            [AssistantAPIClient.PersonaSummary].self,
            from: json
        )

        XCTAssertEqual(personas.count, 1)
        XCTAssertEqual(personas[0].id, "default")
        XCTAssertEqual(personas[0].name, "Default")
    }

    func testWorkflowSummaryDecoding() throws {
        let json = """
        [
            {
                "id": "wf-1",
                "name": "Morning Briefing",
                "description": "Daily summary",
                "active": true,
                "node_count": 3,
                "edge_count": 2,
                "trigger_count": 1,
                "action_count": 2,
                "max_steps": 100,
                "max_visits_per_node": 1,
                "created_at": "2025-01-01T00:00:00Z",
                "updated_at": "2025-01-01T00:00:00Z"
            }
        ]
        """.data(using: .utf8)!

        let workflows = try JSONDecoder().decode(
            [AssistantAPIClient.WorkflowSummary].self,
            from: json
        )

        XCTAssertEqual(workflows.count, 1)
        XCTAssertEqual(workflows[0].name, "Morning Briefing")
        XCTAssertTrue(workflows[0].active)
    }

    func testConversationSummaryDecoding() throws {
        let json = """
        [
            {
                "id": "conv-1",
                "title": "Chat about cooking",
                "updated_at": "2025-01-15T12:00:00Z"
            }
        ]
        """.data(using: .utf8)!

        let conversations = try JSONDecoder().decode(
            [AssistantAPIClient.ConversationSummary].self,
            from: json
        )

        XCTAssertEqual(conversations.count, 1)
        XCTAssertEqual(conversations[0].title, "Chat about cooking")
        XCTAssertEqual(conversations[0].updatedAt, "2025-01-15T12:00:00Z")
    }

    func testWorkflowRunPreviewDecoding() throws {
        let json = """
        {
            "workflow_id": "wf-1",
            "run_id": "run-abc",
            "status": "accepted"
        }
        """.data(using: .utf8)!

        let preview = try JSONDecoder().decode(
            AssistantAPIClient.WorkflowRunPreview.self,
            from: json
        )

        XCTAssertEqual(preview.workflowId, "wf-1")
        XCTAssertEqual(preview.runId, "run-abc")
        XCTAssertEqual(preview.status, "accepted")
    }
}
