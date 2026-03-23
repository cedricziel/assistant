import { Application } from "@hotwired/stimulus";

import WorkflowEditorController from "controllers/workflow-editor";
import WorkflowSecretsController from "controllers/workflow-secrets";

var application = Application.start();

application.register("workflow-editor", WorkflowEditorController);
application.register("workflow-secrets", WorkflowSecretsController);

window.Stimulus = application;
