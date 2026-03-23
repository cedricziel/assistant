import { Application } from "@hotwired/stimulus";

import WorkflowEditorController from "controllers/workflow-editor";
import WorkflowViewModeController from "controllers/workflow-view-mode";
import WorkflowSecretsController from "controllers/workflow-secrets";

var application = Application.start();

application.register("workflow-editor", WorkflowEditorController);
application.register("workflow-view-mode", WorkflowViewModeController);
application.register("workflow-secrets", WorkflowSecretsController);

window.Stimulus = application;
