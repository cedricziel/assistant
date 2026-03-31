import { Application } from "@hotwired/stimulus";

import WorkflowEditorController from "controllers/workflow-editor";
import WorkflowViewModeController from "controllers/workflow-view-mode";
import WorkflowSecretsController from "controllers/workflow-secrets";
import ConversationListController from "controllers/conversation-list";
import MessagePanelController from "controllers/message-panel";

var application = Application.start();

application.register("workflow-editor", WorkflowEditorController);
application.register("workflow-view-mode", WorkflowViewModeController);
application.register("workflow-secrets", WorkflowSecretsController);
application.register("conversation-list", ConversationListController);
application.register("message-panel", MessagePanelController);

window.Stimulus = application;
