import { Controller } from "@hotwired/stimulus";

const VIEW_PREF_PREFIX = "workflow-editor-view:";

export default class extends Controller {
  static targets = [
    "flowPanel",
    "graphPanel",
    "runsPanel",
    "stepsPanel",
    "flowToggle",
    "graphToggle",
    "runsToggle",
    "stepsToggle",
  ];

  static values = {
    workflowId: String,
    mode: String,
  };

  connect() {
    this.resizeHandler = this.onResize.bind(this);
    window.addEventListener("resize", this.resizeHandler);
    this.applyInitialMode();
  }

  disconnect() {
    window.removeEventListener("resize", this.resizeHandler);
  }

  switchMode(event) {
    const mode = event.currentTarget.getAttribute("data-mode");
    if (!mode) return;
    this.setMode(mode, true);
  }

  onResize() {
    if (window.innerWidth < 768 && this.modeValue === "graph") {
      this.setMode("flow", false);
    }
    this.updateModeUi();
  }

  viewStorageKey() {
    if (!this.hasWorkflowIdValue) return "";
    return VIEW_PREF_PREFIX + this.workflowIdValue;
  }

  applyInitialMode() {
    if (!this.hasFlowPanelTarget || !this.hasGraphPanelTarget) return;
    if (window.innerWidth < 768) {
      this.setMode("flow", false);
      return;
    }

    let preferred = "";
    if (this.hasWorkflowIdValue) {
      preferred = localStorage.getItem(this.viewStorageKey()) || "";
    }

    const defaultMode =
      preferred || (this.hasModeValue ? this.modeValue : "flow");
    this.setMode(defaultMode, false);
  }

  setMode(mode, persist) {
    if (!this.hasFlowPanelTarget || !this.hasGraphPanelTarget) return;
    const normalized =
      mode === "graph" || mode === "runs" || mode === "steps" ? mode : "flow";
    const enforced =
      window.innerWidth < 768 && normalized === "graph" ? "flow" : normalized;
    this.modeValue = enforced;

    if (persist && this.hasWorkflowIdValue) {
      localStorage.setItem(this.viewStorageKey(), enforced);
    }

    this.updateModeUi();
  }

  updateModeUi() {
    if (!this.hasFlowPanelTarget || !this.hasGraphPanelTarget) return;
    const desktopMode =
      this.modeValue === "graph" ||
      this.modeValue === "runs" ||
      this.modeValue === "steps"
        ? this.modeValue
        : "flow";
    const mode = window.innerWidth >= 768 ? desktopMode : "flow";

    this.flowPanelTarget.hidden = mode !== "flow";
    this.graphPanelTarget.hidden = mode !== "graph";
    if (this.hasRunsPanelTarget) {
      this.runsPanelTarget.hidden = mode !== "runs";
    }
    if (this.hasStepsPanelTarget) {
      this.stepsPanelTarget.hidden = mode !== "steps";
    }

    if (this.hasFlowToggleTarget) {
      const active = mode === "flow";
      this.flowToggleTarget.classList.toggle("is-active", active);
      this.flowToggleTarget.setAttribute(
        "aria-pressed",
        active ? "true" : "false",
      );
    }

    if (this.hasGraphToggleTarget) {
      const active = mode === "graph";
      const disabled = window.innerWidth < 768;
      this.graphToggleTarget.classList.toggle("is-active", active);
      this.graphToggleTarget.setAttribute(
        "aria-pressed",
        active ? "true" : "false",
      );
      this.graphToggleTarget.disabled = disabled;
      this.graphToggleTarget.setAttribute(
        "title",
        disabled ? "Graph view is available on tablet and desktop" : "",
      );
    }

    if (this.hasRunsToggleTarget) {
      const active = mode === "runs";
      this.runsToggleTarget.classList.toggle("is-active", active);
      this.runsToggleTarget.setAttribute(
        "aria-pressed",
        active ? "true" : "false",
      );
    }

    if (this.hasStepsToggleTarget) {
      const active = mode === "steps";
      this.stepsToggleTarget.classList.toggle("is-active", active);
      this.stepsToggleTarget.setAttribute(
        "aria-pressed",
        active ? "true" : "false",
      );
    }
  }
}
