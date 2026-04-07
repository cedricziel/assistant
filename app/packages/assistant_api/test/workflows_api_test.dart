import 'package:test/test.dart';
import 'package:assistant_api/assistant_api.dart';


/// tests for WorkflowsApi
void main() {
  final instance = AssistantApi().getWorkflowsApi();

  group(WorkflowsApi, () {
    // `POST /api/workflows/{id}/activate` — activate a workflow.
    //
    //Future<WorkflowDetail> activateWorkflow(String id) async
    test('test activateWorkflow', () async {
      // TODO
    });

    // `POST /api/workflows` — create a new workflow.
    //
    //Future<WorkflowDetail> createWorkflow(WorkflowUpsertRequest workflowUpsertRequest) async
    test('test createWorkflow', () async {
      // TODO
    });

    // `POST /api/workflows/{id}/deactivate` — deactivate a workflow.
    //
    //Future<WorkflowDetail> deactivateWorkflow(String id) async
    test('test deactivateWorkflow', () async {
      // TODO
    });

    // `DELETE /api/workflows/{id}` — delete a workflow.
    //
    //Future deleteWorkflow(String id) async
    test('test deleteWorkflow', () async {
      // TODO
    });

    // `GET /api/workflows/{id}` — fetch a workflow.
    //
    //Future<WorkflowDetail> getWorkflow(String id) async
    test('test getWorkflow', () async {
      // TODO
    });

    // `GET /api/workflows/{id}/runs/{run_id}` — workflow run detail with steps.
    //
    //Future<WorkflowRunDetail> getWorkflowRun(String id, String runId) async
    test('test getWorkflowRun', () async {
      // TODO
    });

    // `GET /api/workflows/{id}/webhook-secrets` — reveal webhook URL and token.
    //
    //Future<WorkflowWebhookSecrets> getWorkflowWebhookSecrets(String id) async
    test('test getWorkflowWebhookSecrets', () async {
      // TODO
    });

    // `GET /api/workflows/{id}/runs` — list recent runs (up to 50).
    //
    //Future<BuiltList<WorkflowRunSummary>> listWorkflowRuns(String id) async
    test('test listWorkflowRuns', () async {
      // TODO
    });

    // `GET /api/workflows` — list all workflows.
    //
    //Future<BuiltList<WorkflowSummary>> listWorkflows() async
    test('test listWorkflows', () async {
      // TODO
    });

    // `POST /api/workflows/{id}/test-run` — queue a manual test run.
    //
    //Future<WorkflowRunPreview> testRunWorkflow(String id) async
    test('test testRunWorkflow', () async {
      // TODO
    });

    // `PUT /api/workflows/{id}` — update a workflow.
    //
    //Future<WorkflowDetail> updateWorkflow(String id, WorkflowUpsertRequest workflowUpsertRequest) async
    test('test updateWorkflow', () async {
      // TODO
    });

  });
}
