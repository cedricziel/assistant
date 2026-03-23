import { Application } from "@hotwired/stimulus";

import WorkflowSecretsController from "controllers/workflow-secrets";

var application = Application.start();

application.register("workflow-secrets", WorkflowSecretsController);

window.Stimulus = application;
