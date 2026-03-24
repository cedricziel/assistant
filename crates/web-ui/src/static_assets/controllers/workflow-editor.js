import { Controller } from "@hotwired/stimulus";

const GRAPH_DRAFT_PREFIX = "workflow-editor-draft:";

function parseGraph(raw) {
  if (!raw || !raw.trim()) {
    return {
      version: 1,
      nodes: [
        {
          id: "start",
          kind: "trigger",
          config: { type: "manual" },
        },
      ],
      edges: [],
      execution: {
        max_steps: 200,
        max_visits_per_node: 25,
      },
    };
  }

  const parsed = JSON.parse(raw);
  if (!parsed.execution) {
    parsed.execution = { max_steps: 200, max_visits_per_node: 25 };
  }
  if (!Array.isArray(parsed.nodes)) parsed.nodes = [];
  if (!Array.isArray(parsed.edges)) parsed.edges = [];
  if (!parsed.version) parsed.version = 1;
  return parsed;
}

function normalizeKind(kind) {
  if (kind === "trigger" || kind === "condition" || kind === "action") {
    return kind;
  }
  return "action";
}

function titleFromNode(node) {
  if (node.kind === "trigger") return "Something starts this";
  if (node.kind === "condition") return "Choose what happens next";
  return "Do this step";
}

function subtitleFromNode(node) {
  const configType =
    node.config && typeof node.config.type === "string" ? node.config.type : "";
  if (!configType) return "Add a short description in details";
  return configType.replace(/[_-]+/g, " ");
}

function topologicalOrder(graph) {
  const indegree = new Map();
  const outgoing = new Map();
  graph.nodes.forEach(function (node) {
    indegree.set(node.id, 0);
    outgoing.set(node.id, []);
  });

  graph.edges.forEach(function (edge) {
    if (!indegree.has(edge.to) || !outgoing.has(edge.from)) return;
    indegree.set(edge.to, (indegree.get(edge.to) || 0) + 1);
    outgoing.get(edge.from).push(edge.to);
  });

  const queue = [];
  graph.nodes.forEach(function (node) {
    if ((indegree.get(node.id) || 0) === 0) {
      queue.push(node.id);
    }
  });

  const orderedIds = [];
  while (queue.length > 0) {
    const id = queue.shift();
    orderedIds.push(id);
    (outgoing.get(id) || []).forEach(function (to) {
      const next = (indegree.get(to) || 0) - 1;
      indegree.set(to, next);
      if (next === 0) queue.push(to);
    });
  }

  graph.nodes.forEach(function (node) {
    if (!orderedIds.includes(node.id)) {
      orderedIds.push(node.id);
    }
  });

  const byId = new Map(graph.nodes.map((node) => [node.id, node]));
  return orderedIds.map((id) => byId.get(id)).filter(Boolean);
}

export default class extends Controller {
  static targets = [
    "textarea",
    "error",
    "nodeRows",
    "svg",
    "flow",
    "status",
    "saveButton",
    "nodeIdInput",
    "nodeKindInput",
    "nodeTypeInput",
    "edgeFromInput",
    "edgeToInput",
    "edgeOnInput",
  ];

  static values = {
    workflowId: String,
  };

  connect() {
    this.graph = null;
    this.isDirty = false;
    this.isSaving = false;
    this.pendingConnectSource = null;
    this.refreshTimer = null;
    this.beforeUnloadHandler = this.beforeUnload.bind(this);

    window.addEventListener("beforeunload", this.beforeUnloadHandler);

    if (this.hasWorkflowIdValue) {
      const draft = localStorage.getItem(this.draftStorageKey());
      if (draft && draft.trim() && this.hasTextareaTarget) {
        this.textareaTarget.value = draft;
        this.markDirty();
      }
    }

    this.refreshGraphFromText();
  }

  disconnect() {
    window.removeEventListener("beforeunload", this.beforeUnloadHandler);
    if (this.refreshTimer !== null) {
      window.clearTimeout(this.refreshTimer);
      this.refreshTimer = null;
    }
  }

  onGraphInput() {
    this.markDirty();
    this.persistDraft();
    this.scheduleRefresh();
  }

  addStep(event) {
    const sourceId = event?.currentTarget?.getAttribute("data-source-id") || "";
    this.addStepWithPrompt(sourceId);
  }

  beginConnect(event) {
    const sourceId = event?.currentTarget?.getAttribute("data-source-id") || "";
    if (sourceId) {
      this.pendingConnectSource = sourceId;
      this.setStatus(
        `Connecting from '${sourceId}'. Click destination step.`,
        false,
      );
      return;
    }

    this.pendingConnectSource = "";
    this.setStatus("Connection mode active. Click source step.", false);
  }

  cancelConnect() {
    this.pendingConnectSource = null;
    this.setStatus("Connection mode cancelled.", false);
  }

  pickConnectNode(nodeId) {
    if (this.pendingConnectSource === null) return;

    if (this.pendingConnectSource === "") {
      this.pendingConnectSource = nodeId;
      this.setStatus(
        `Source '${nodeId}' selected. Click destination step.`,
        false,
      );
      this.refreshGraphFromText();
      return;
    }

    if (this.pendingConnectSource === nodeId) {
      this.setStatus("Pick a different destination step.", true);
      return;
    }

    const labelRaw = window.prompt("Connection label (optional)", "") || "";
    const ok = this.addEdgeByIds(
      this.pendingConnectSource,
      nodeId,
      labelRaw.trim(),
    );
    if (ok) {
      this.pendingConnectSource = null;
    }
  }

  nextSuggestedNodeId(graph, prefix) {
    const used = new Set(
      graph.nodes.map(function (node) {
        return node.id;
      }),
    );

    let i = 1;
    while (used.has(prefix + "-" + i)) i += 1;
    return prefix + "-" + i;
  }

  promptForNode(defaultId) {
    const rawId = window.prompt("Step name", defaultId);
    if (rawId === null) return null;
    const nodeId = rawId.trim();
    if (!nodeId) {
      this.setError("Step name cannot be empty");
      return null;
    }

    const kindRaw = window.prompt(
      "Step kind: trigger, action, or condition",
      "action",
    );
    if (kindRaw === null) return null;

    const typeRaw = window.prompt(
      "Step type (example: assistant_turn, http_request, manual)",
      "custom",
    );
    if (typeRaw === null) return null;

    return {
      id: nodeId,
      kind: normalizeKind(kindRaw.trim().toLowerCase()),
      config: {
        type: (typeRaw.trim() || "custom").toLowerCase(),
      },
    };
  }

  addStepWithPrompt(sourceId) {
    const graph = this.currentGraph();
    if (!graph) return;

    const node = this.promptForNode(this.nextSuggestedNodeId(graph, "step"));
    if (!node) return;

    if (
      graph.nodes.some(function (existing) {
        return existing.id === node.id;
      })
    ) {
      this.setError("That step name already exists");
      return;
    }

    graph.nodes.push(node);

    if (sourceId) {
      const labelRaw = window.prompt("Connection label (optional)", "") || "";
      const edgeAdded = this.pushEdge(
        graph,
        sourceId,
        node.id,
        labelRaw.trim(),
      );
      if (!edgeAdded) {
        graph.nodes = graph.nodes.filter(function (existing) {
          return existing.id !== node.id;
        });
        return;
      }
    }

    this.writeGraph(graph);
    this.markDirty();
    this.persistDraft();
    this.refreshGraphFromText();
    this.setStatus(`Added step '${node.id}'.`, false);
  }

  pushEdge(graph, from, to, label) {
    const knownIds = new Set(
      graph.nodes.map(function (node) {
        return node.id;
      }),
    );

    if (!knownIds.has(from) || !knownIds.has(to)) {
      this.setError("Both steps must already exist");
      return false;
    }

    if (from === to) {
      this.setError("A step cannot connect to itself");
      return false;
    }

    const normalizedLabel = label || null;
    const duplicate = graph.edges.some(function (edge) {
      return (
        edge.from === from &&
        edge.to === to &&
        (edge.on || null) === normalizedLabel
      );
    });
    if (duplicate) {
      this.setError("That connection already exists");
      return false;
    }

    graph.edges.push({
      from,
      to,
      on: normalizedLabel,
    });
    return true;
  }

  addEdgeByIds(from, to, label) {
    const graph = this.currentGraph();
    if (!graph) return false;

    if (!this.pushEdge(graph, from, to, label)) return false;

    this.writeGraph(graph);
    this.markDirty();
    this.persistDraft();
    this.refreshGraphFromText();
    this.setStatus(`Connected '${from}' to '${to}'.`, false);
    return true;
  }

  addNode() {
    const graph = this.currentGraph();
    if (!graph || !this.hasNodeIdInputTarget || !this.hasNodeKindInputTarget)
      return;

    const nodeId = this.nodeIdInputTarget.value.trim();
    if (!nodeId) {
      this.setError("Name this step before adding it");
      return;
    }

    if (
      graph.nodes.some(function (node) {
        return node.id === nodeId;
      })
    ) {
      this.setError("That step name already exists");
      return;
    }

    const nodeType = this.hasNodeTypeInputTarget
      ? this.nodeTypeInputTarget.value.trim() || "custom"
      : "custom";

    graph.nodes.push({
      id: nodeId,
      kind: normalizeKind(this.nodeKindInputTarget.value),
      config: { type: nodeType },
    });

    this.writeGraph(graph);
    this.markDirty();
    this.persistDraft();
    this.refreshGraphFromText();
    this.nodeIdInputTarget.value = "";
  }

  addEdge() {
    const graph = this.currentGraph();
    if (!graph || !this.hasEdgeFromInputTarget || !this.hasEdgeToInputTarget)
      return;

    const from = this.edgeFromInputTarget.value.trim();
    const to = this.edgeToInputTarget.value.trim();
    const onLabel = this.hasEdgeOnInputTarget
      ? this.edgeOnInputTarget.value.trim()
      : "";

    if (!from || !to) {
      this.setError("Choose both steps before connecting");
      return;
    }

    const knownIds = new Set(
      graph.nodes.map(function (node) {
        return node.id;
      }),
    );
    if (!knownIds.has(from) || !knownIds.has(to)) {
      this.setError("Both step names must already exist");
      return;
    }

    if (!this.pushEdge(graph, from, to, onLabel)) return;

    this.writeGraph(graph);
    this.markDirty();
    this.persistDraft();
    this.refreshGraphFromText();
  }

  save() {
    this.saveDraftToServer();
  }

  navigate(event) {
    if (!this.isDirty) return;
    const href = event.currentTarget.getAttribute("href");
    if (!href) return;
    event.preventDefault();

    if (!window.confirm("You have unsaved changes. Save before leaving?")) {
      window.location.href = href;
      return;
    }

    this.saveDraftToServer().then(function (saved) {
      if (saved) {
        window.location.href = href;
      }
    });
  }

  beforeUnload(event) {
    if (!this.isDirty) return;
    event.preventDefault();
    event.returnValue = "";
  }

  draftStorageKey() {
    if (!this.hasWorkflowIdValue) return "";
    return GRAPH_DRAFT_PREFIX + this.workflowIdValue;
  }

  persistDraft() {
    if (!this.hasWorkflowIdValue || !this.hasTextareaTarget) return;
    localStorage.setItem(this.draftStorageKey(), this.textareaTarget.value);
  }

  clearDraft() {
    if (!this.hasWorkflowIdValue) return;
    localStorage.removeItem(this.draftStorageKey());
  }

  currentGraph() {
    if (this.graph) return this.graph;
    return this.refreshGraphFromText();
  }

  writeGraph(graph) {
    if (!this.hasTextareaTarget) return;
    this.textareaTarget.value = JSON.stringify(graph, null, 2);
    this.clearError();
  }

  scheduleRefresh() {
    if (this.refreshTimer !== null) {
      window.clearTimeout(this.refreshTimer);
    }
    this.refreshTimer = window.setTimeout(() => {
      this.refreshTimer = null;
      this.refreshGraphFromText();
    }, 120);
  }

  refreshGraphFromText() {
    if (!this.hasTextareaTarget) return null;
    try {
      const graph = parseGraph(this.textareaTarget.value);
      this.graph = graph;
      this.renderNodeTable(graph);
      this.renderFlow(graph);
      this.renderGraph(graph);
      this.clearError();
      return graph;
    } catch (error) {
      this.setError(
        "Technical details are invalid JSON. Please fix and try again.",
      );
      return null;
    }
  }

  renderNodeTable(graph) {
    if (!this.hasNodeRowsTarget) return;
    this.nodeRowsTarget.innerHTML = "";

    graph.nodes.forEach((node, index) => {
      const row = document.createElement("tr");

      const step = document.createElement("td");
      step.textContent = String(index + 1);

      const purpose = document.createElement("td");
      purpose.textContent = titleFromNode(node);

      const details = document.createElement("td");
      const code = document.createElement("code");
      code.textContent = node.id;
      details.appendChild(code);

      row.append(step, purpose, details);
      this.nodeRowsTarget.appendChild(row);
    });
  }

  renderFlow(graph) {
    if (!this.hasFlowTarget) return;
    this.flowTarget.innerHTML = "";

    const nodes = topologicalOrder(graph);
    const outgoing = new Map();
    graph.edges.forEach(function (edge) {
      if (!outgoing.has(edge.from)) outgoing.set(edge.from, []);
      outgoing.get(edge.from).push(edge);
    });

    nodes.forEach((node, index) => {
      const card = document.createElement("article");
      card.className = "wf-flow-card";

      const title = document.createElement("h4");
      title.textContent = index + 1 + ") " + titleFromNode(node);
      card.appendChild(title);

      const subtitle = document.createElement("p");
      subtitle.textContent = subtitleFromNode(node);
      card.appendChild(subtitle);

      const actions = document.createElement("div");
      actions.className = "wf-flow-actions";

      const addButton = document.createElement("button");
      addButton.type = "button";
      addButton.className = "ghost-btn wf-mini-btn";
      addButton.textContent = "+ Next step";
      addButton.addEventListener("click", (event) => {
        event.stopPropagation();
        this.addStepWithPrompt(node.id);
      });
      actions.appendChild(addButton);

      const connectButton = document.createElement("button");
      connectButton.type = "button";
      connectButton.className = "ghost-btn wf-mini-btn";
      connectButton.textContent = "Connect";
      connectButton.addEventListener("click", (event) => {
        event.stopPropagation();
        this.pendingConnectSource = node.id;
        this.setStatus(
          `Connecting from '${node.id}'. Click destination step.`,
          false,
        );
        this.refreshGraphFromText();
      });
      actions.appendChild(connectButton);

      card.appendChild(actions);

      if (this.pendingConnectSource === node.id) {
        card.classList.add("is-connect-source");
      }

      card.addEventListener("click", (event) => {
        if (
          event.target instanceof HTMLElement &&
          event.target.closest("button")
        ) {
          return;
        }
        this.pickConnectNode(node.id);
      });

      const next = outgoing.get(node.id) || [];
      if (next.length > 0) {
        const hint = document.createElement("div");
        hint.className = "wf-flow-next";
        next.forEach(function (edge) {
          const chip = document.createElement("span");
          chip.className = "wf-flow-chip";
          const label = edge.on ? edge.on + " -> " : "Then -> ";
          chip.textContent = label + edge.to;
          hint.appendChild(chip);
        });
        card.appendChild(hint);
      }

      this.flowTarget.appendChild(card);
    });
  }

  renderGraph(graph) {
    if (!this.hasSvgTarget) return;
    this.svgTarget.innerHTML = "";

    const nodes = topologicalOrder(graph);
    if (nodes.length === 0) return;

    const indegree = new Map();
    const level = new Map();
    const outgoing = new Map();
    nodes.forEach(function (node) {
      indegree.set(node.id, 0);
      outgoing.set(node.id, []);
      level.set(node.id, 0);
    });

    graph.edges.forEach(function (edge) {
      if (!indegree.has(edge.to) || !outgoing.has(edge.from)) return;
      indegree.set(edge.to, (indegree.get(edge.to) || 0) + 1);
      outgoing.get(edge.from).push(edge.to);
    });

    const queue = [];
    nodes.forEach(function (node) {
      if ((indegree.get(node.id) || 0) === 0) queue.push(node.id);
    });

    while (queue.length > 0) {
      const nodeId = queue.shift();
      const nodeLevel = level.get(nodeId) || 0;
      (outgoing.get(nodeId) || []).forEach(function (nextId) {
        level.set(nextId, Math.max(level.get(nextId) || 0, nodeLevel + 1));
        const nextIn = (indegree.get(nextId) || 0) - 1;
        indegree.set(nextId, nextIn);
        if (nextIn === 0) queue.push(nextId);
      });
    }

    const columns = new Map();
    nodes.forEach(function (node) {
      const nodeLevel = level.get(node.id) || 0;
      if (!columns.has(nodeLevel)) columns.set(nodeLevel, []);
      columns.get(nodeLevel).push(node.id);
    });

    const colKeys = Array.from(columns.keys()).sort(function (a, b) {
      return a - b;
    });
    const nodeWidth = 180;
    const nodeHeight = 64;
    const gapX = 80;
    const gapY = 44;

    const positions = new Map();
    colKeys.forEach(function (col, colIndex) {
      const ids = columns.get(col) || [];
      const colHeight =
        ids.length * nodeHeight + Math.max(0, ids.length - 1) * gapY;
      ids.forEach(function (nodeId, rowIndex) {
        const x = 40 + colIndex * (nodeWidth + gapX);
        const y =
          Math.max(40, 220 - colHeight / 2) + rowIndex * (nodeHeight + gapY);
        positions.set(nodeId, { x: x, y: y });
      });
    });

    const width = Math.max(980, colKeys.length * (nodeWidth + gapX) + 120);
    const maxY = Array.from(positions.values()).reduce(function (max, pos) {
      return Math.max(max, pos.y + nodeHeight);
    }, 420);
    const height = Math.max(420, maxY + 40);
    this.svgTarget.setAttribute("viewBox", "0 0 " + width + " " + height);

    graph.edges.forEach((edge) => {
      const from = positions.get(edge.from);
      const to = positions.get(edge.to);
      if (!from || !to) return;

      const startX = from.x + nodeWidth;
      const startY = from.y + nodeHeight / 2;
      const endX = to.x;
      const endY = to.y + nodeHeight / 2;
      const controlX = (startX + endX) / 2;

      const path = document.createElementNS(
        "http://www.w3.org/2000/svg",
        "path",
      );
      const d =
        "M " +
        startX +
        " " +
        startY +
        " C " +
        controlX +
        " " +
        startY +
        ", " +
        controlX +
        " " +
        endY +
        ", " +
        endX +
        " " +
        endY;
      path.setAttribute("d", d);
      path.setAttribute("stroke", "#5ea2e6");
      path.setAttribute("fill", "none");
      path.setAttribute("stroke-width", "2");
      this.svgTarget.appendChild(path);
    });

    nodes.forEach((node) => {
      const pos = positions.get(node.id);
      if (!pos) return;

      const group = document.createElementNS("http://www.w3.org/2000/svg", "g");

      const rect = document.createElementNS(
        "http://www.w3.org/2000/svg",
        "rect",
      );
      rect.setAttribute("x", String(pos.x));
      rect.setAttribute("y", String(pos.y));
      rect.setAttribute("width", String(nodeWidth));
      rect.setAttribute("height", String(nodeHeight));
      rect.setAttribute("rx", "12");
      rect.setAttribute(
        "fill",
        node.kind === "trigger" ? "#1b3452" : "#13263d",
      );
      rect.setAttribute(
        "stroke",
        this.pendingConnectSource === node.id ? "#fbbf24" : "#6ec6ff",
      );
      rect.setAttribute(
        "stroke-width",
        this.pendingConnectSource === node.id ? "2" : "1.2",
      );
      group.appendChild(rect);

      const title = document.createElementNS(
        "http://www.w3.org/2000/svg",
        "text",
      );
      title.setAttribute("x", String(pos.x + nodeWidth / 2));
      title.setAttribute("y", String(pos.y + 28));
      title.setAttribute("fill", "#e5e9f0");
      title.setAttribute("font-size", "12");
      title.setAttribute("text-anchor", "middle");
      title.textContent = titleFromNode(node);
      group.appendChild(title);

      const subtitle = document.createElementNS(
        "http://www.w3.org/2000/svg",
        "text",
      );
      subtitle.setAttribute("x", String(pos.x + nodeWidth / 2));
      subtitle.setAttribute("y", String(pos.y + 46));
      subtitle.setAttribute("fill", "#8aa5d8");
      subtitle.setAttribute("font-size", "11");
      subtitle.setAttribute("text-anchor", "middle");
      subtitle.textContent = node.id;
      group.appendChild(subtitle);

      const connectHandle = document.createElementNS(
        "http://www.w3.org/2000/svg",
        "circle",
      );
      connectHandle.setAttribute("cx", String(pos.x + nodeWidth + 8));
      connectHandle.setAttribute("cy", String(pos.y + nodeHeight / 2));
      connectHandle.setAttribute("r", "8");
      connectHandle.setAttribute("fill", "#0b2d4d");
      connectHandle.setAttribute("stroke", "#6ec6ff");
      connectHandle.setAttribute("stroke-width", "1.2");
      connectHandle.style.cursor = "pointer";
      connectHandle.addEventListener("click", (event) => {
        event.stopPropagation();
        this.pendingConnectSource = node.id;
        this.setStatus(
          `Connecting from '${node.id}'. Click destination node.`,
          false,
        );
        this.refreshGraphFromText();
      });
      group.appendChild(connectHandle);

      const addHandle = document.createElementNS(
        "http://www.w3.org/2000/svg",
        "circle",
      );
      addHandle.setAttribute("cx", String(pos.x + nodeWidth - 12));
      addHandle.setAttribute("cy", String(pos.y + 12));
      addHandle.setAttribute("r", "9");
      addHandle.setAttribute("fill", "#163453");
      addHandle.setAttribute("stroke", "#7aa2ff");
      addHandle.setAttribute("stroke-width", "1.2");
      addHandle.style.cursor = "pointer";
      addHandle.addEventListener("click", (event) => {
        event.stopPropagation();
        this.addStepWithPrompt(node.id);
      });
      group.appendChild(addHandle);

      const plus = document.createElementNS(
        "http://www.w3.org/2000/svg",
        "text",
      );
      plus.setAttribute("x", String(pos.x + nodeWidth - 12));
      plus.setAttribute("y", String(pos.y + 16));
      plus.setAttribute("fill", "#cfe1ff");
      plus.setAttribute("font-size", "12");
      plus.setAttribute("text-anchor", "middle");
      plus.textContent = "+";
      plus.style.pointerEvents = "none";
      group.appendChild(plus);

      group.style.cursor = "pointer";
      group.addEventListener("click", () => {
        this.pickConnectNode(node.id);
      });

      this.svgTarget.appendChild(group);
    });
  }

  setStatus(message, isError) {
    if (!this.hasStatusTarget) return;
    this.statusTarget.textContent = message;
    this.statusTarget.style.color = isError ? "#ff9b9b" : "#8aa5d8";
  }

  setSaveButtonState(saving) {
    if (!this.hasSaveButtonTarget) return;
    this.saveButtonTarget.disabled = saving;
    this.saveButtonTarget.textContent = saving ? "Saving..." : "Save changes";
  }

  markDirty() {
    this.isDirty = true;
    this.setStatus("Unsaved changes", false);
  }

  clearError() {
    if (!this.hasErrorTarget) return;
    this.errorTarget.textContent = "";
  }

  setError(message) {
    if (!this.hasErrorTarget) return;
    this.errorTarget.textContent = message;
  }

  async saveDraftToServer() {
    if (this.isSaving || !this.hasWorkflowIdValue) return false;

    const graph = this.currentGraph();
    if (!graph) {
      this.setStatus("Cannot save while technical details are invalid", true);
      return false;
    }

    this.isSaving = true;
    this.setSaveButtonState(true);

    try {
      const workflowRes = await fetch("/api/workflows/" + this.workflowIdValue);
      if (!workflowRes.ok) throw new Error("Failed to load workflow");
      const workflow = await workflowRes.json();

      const saveRes = await fetch("/api/workflows/" + this.workflowIdValue, {
        method: "PUT",
        headers: { "Content-Type": "application/json" },
        body: JSON.stringify({
          name: workflow.name,
          description: workflow.description,
          active: workflow.active,
          graph: graph,
        }),
      });

      if (!saveRes.ok) throw new Error("Save failed");
      this.isDirty = false;
      this.clearDraft();
      this.setStatus("Saved", false);
      return true;
    } catch (error) {
      this.setStatus(String(error), true);
      return false;
    } finally {
      this.isSaving = false;
      this.setSaveButtonState(false);
    }
  }
}
