// Workflow editor: keeps JSON graph, node list, and SVG preview in sync.

function parseGraph(raw) {
  if (!raw || !raw.trim()) {
    return {
      version: 1,
      nodes: [
        {
          id: "trigger_1",
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

function writeGraph(textarea, graph, errorBox) {
  textarea.value = JSON.stringify(graph, null, 2);
  if (errorBox) {
    errorBox.textContent = "";
  }
}

function renderNodeTable(graph, tbody) {
  if (!tbody) return;
  tbody.innerHTML = "";
  graph.nodes.forEach(function (node) {
    const tr = document.createElement("tr");

    const idTd = document.createElement("td");
    idTd.textContent = node.id;

    const kindTd = document.createElement("td");
    kindTd.textContent = node.kind;

    const configTd = document.createElement("td");
    const code = document.createElement("code");
    code.textContent = JSON.stringify(node.config || {});
    configTd.appendChild(code);

    tr.append(idTd, kindTd, configTd);
    tbody.appendChild(tr);
  });
}

function renderSvg(graph, svg) {
  if (!svg) return;
  svg.innerHTML = "";

  const width = svg.viewBox.baseVal.width || 980;
  const stepX = 180;
  const stepY = 120;
  const cols = Math.max(1, Math.floor(width / stepX));

  const positions = {};
  graph.nodes.forEach(function (node, index) {
    const col = index % cols;
    const row = Math.floor(index / cols);
    positions[node.id] = {
      x: 80 + col * stepX,
      y: 60 + row * stepY,
    };
  });

  graph.edges.forEach(function (edge) {
    const from = positions[edge.from];
    const to = positions[edge.to];
    if (!from || !to) return;

    const path = document.createElementNS("http://www.w3.org/2000/svg", "path");
    const ctrlX = (from.x + to.x) / 2;
    const d =
      "M " +
      from.x +
      " " +
      from.y +
      " C " +
      ctrlX +
      " " +
      from.y +
      ", " +
      ctrlX +
      " " +
      to.y +
      ", " +
      to.x +
      " " +
      to.y;
    path.setAttribute("d", d);
    path.setAttribute("stroke", "#5ea2e6");
    path.setAttribute("fill", "none");
    path.setAttribute("stroke-width", "2");
    svg.appendChild(path);
  });

  graph.nodes.forEach(function (node) {
    const p = positions[node.id];
    if (!p) return;

    const g = document.createElementNS("http://www.w3.org/2000/svg", "g");

    const rect = document.createElementNS("http://www.w3.org/2000/svg", "rect");
    rect.setAttribute("x", String(p.x - 58));
    rect.setAttribute("y", String(p.y - 24));
    rect.setAttribute("width", "116");
    rect.setAttribute("height", "48");
    rect.setAttribute("rx", "10");
    rect.setAttribute("fill", node.kind === "trigger" ? "#1b3452" : "#13263d");
    rect.setAttribute("stroke", "#6ec6ff");
    rect.setAttribute("stroke-width", "1.2");
    g.appendChild(rect);

    const text = document.createElementNS("http://www.w3.org/2000/svg", "text");
    text.setAttribute("x", String(p.x));
    text.setAttribute("y", String(p.y - 4));
    text.setAttribute("fill", "#e5e9f0");
    text.setAttribute("font-size", "12");
    text.setAttribute("text-anchor", "middle");
    text.textContent = node.id;
    g.appendChild(text);

    const sub = document.createElementNS("http://www.w3.org/2000/svg", "text");
    sub.setAttribute("x", String(p.x));
    sub.setAttribute("y", String(p.y + 12));
    sub.setAttribute("fill", "#8aa5d8");
    sub.setAttribute("font-size", "10");
    sub.setAttribute("text-anchor", "middle");
    sub.textContent = node.kind;
    g.appendChild(sub);

    svg.appendChild(g);
  });
}

function bindWorkflowEditor() {
  const textarea = document.getElementById("graphJson");
  if (!textarea) return;

  const errorBox = document.getElementById("graphError");
  const nodeTableBody = document.getElementById("workflowNodeRows");
  const svg = document.getElementById("workflowGraphPreview");
  const saveBtn = document.getElementById("saveGraphBtn");
  const statusBox = document.getElementById("editorStatus");
  const editorMeta = document.getElementById("workflowEditorMeta");
  const workflowId = editorMeta
    ? editorMeta.getAttribute("data-workflow-id")
    : null;
  const storageKey = workflowId ? "workflow-editor-draft:" + workflowId : null;
  let isDirty = false;
  let isSaving = false;
  let refreshTimer = null;
  const saveBtnLabel = saveBtn ? saveBtn.textContent : "Save Graph";

  function setStatus(message, isError) {
    if (!statusBox) return;
    statusBox.textContent = message;
    statusBox.style.color = isError ? "#ff8a8a" : "#8aa5d8";
  }

  function markDirty() {
    isDirty = true;
    setStatus("Unsaved changes", false);
  }

  function setSavingState(saving) {
    isSaving = saving;
    if (!saveBtn) return;
    saveBtn.disabled = saving;
    saveBtn.textContent = saving ? "Saving..." : saveBtnLabel;
  }

  function persistDraft() {
    if (!storageKey) return;
    localStorage.setItem(storageKey, textarea.value);
  }

  function clearDraft() {
    if (!storageKey) return;
    localStorage.removeItem(storageKey);
  }

  function saveDraftToServer() {
    if (isSaving) {
      return Promise.resolve(false);
    }
    if (!workflowId) {
      return Promise.resolve(false);
    }
    const graph = refreshFromTextArea();
    if (!graph) {
      setStatus("Cannot save invalid graph JSON", true);
      return Promise.resolve(false);
    }
    setSavingState(true);

    return fetch("/api/workflows/" + workflowId)
      .then(function (res) {
        if (!res.ok) {
          throw new Error("Failed to load workflow");
        }
        return res.json();
      })
      .then(function (workflow) {
        return fetch("/api/workflows/" + workflowId, {
          method: "PUT",
          headers: { "Content-Type": "application/json" },
          body: JSON.stringify({
            name: workflow.name,
            description: workflow.description,
            active: workflow.active,
            graph: graph,
          }),
        });
      })
      .then(function (res) {
        if (!res.ok) {
          throw new Error("Save failed");
        }
        isDirty = false;
        clearDraft();
        setStatus("Saved", false);
        return true;
      })
      .catch(function (err) {
        setStatus(String(err), true);
        return false;
      })
      .finally(function () {
        setSavingState(false);
      });
  }

  if (storageKey) {
    const draft = localStorage.getItem(storageKey);
    if (draft && draft.trim()) {
      textarea.value = draft;
      markDirty();
    }
  }

  function refreshFromTextArea() {
    try {
      const graph = parseGraph(textarea.value);
      renderNodeTable(graph, nodeTableBody);
      renderSvg(graph, svg);
      if (errorBox) errorBox.textContent = "";
      return graph;
    } catch (err) {
      if (errorBox) {
        errorBox.textContent = "Invalid graph JSON: " + err;
      }
      return null;
    }
  }

  function scheduleRefresh() {
    if (refreshTimer !== null) {
      clearTimeout(refreshTimer);
    }
    refreshTimer = window.setTimeout(function () {
      refreshTimer = null;
      refreshFromTextArea();
    }, 120);
  }

  const addNodeBtn = document.getElementById("addNodeBtn");
  if (addNodeBtn) {
    addNodeBtn.addEventListener("click", function () {
      const graph = refreshFromTextArea();
      if (!graph) return;

      const idInput = document.getElementById("nodeIdInput");
      const kindInput = document.getElementById("nodeKindInput");
      const typeInput = document.getElementById("nodeTypeInput");
      if (!idInput || !kindInput || !typeInput) return;

      const nodeId = idInput.value.trim();
      if (!nodeId) return;
      if (
        graph.nodes.some(function (n) {
          return n.id === nodeId;
        })
      ) {
        if (errorBox)
          errorBox.textContent = "Node id already exists: " + nodeId;
        return;
      }

      graph.nodes.push({
        id: nodeId,
        kind: kindInput.value,
        config: { type: typeInput.value.trim() || "custom" },
      });
      writeGraph(textarea, graph, errorBox);
      markDirty();
      persistDraft();
      refreshFromTextArea();
      idInput.value = "";
    });
  }

  const addEdgeBtn = document.getElementById("addEdgeBtn");
  if (addEdgeBtn) {
    addEdgeBtn.addEventListener("click", function () {
      const graph = refreshFromTextArea();
      if (!graph) return;

      const fromInput = document.getElementById("edgeFromInput");
      const toInput = document.getElementById("edgeToInput");
      const onInput = document.getElementById("edgeOnInput");
      if (!fromInput || !toInput || !onInput) return;

      const from = fromInput.value.trim();
      const to = toInput.value.trim();
      if (!from || !to) return;

      const knownNodeIds = new Set(
        graph.nodes.map(function (node) {
          return node.id;
        }),
      );
      if (!knownNodeIds.has(from) || !knownNodeIds.has(to)) {
        if (errorBox) {
          errorBox.textContent =
            "Both edge endpoints must reference existing nodes";
        }
        return;
      }

      graph.edges.push({
        from: from,
        to: to,
        on: onInput.value.trim() || null,
      });
      writeGraph(textarea, graph, errorBox);
      markDirty();
      persistDraft();
      refreshFromTextArea();
    });
  }

  if (saveBtn) {
    saveBtn.addEventListener("click", function () {
      saveDraftToServer();
    });
  }

  const navLinks = document.querySelectorAll("a[data-editor-nav='true']");
  navLinks.forEach(function (link) {
    link.addEventListener("click", function (event) {
      if (!isDirty) return;
      event.preventDefault();
      if (!confirm("You have unsaved changes. Save before leaving?")) {
        window.location.href = link.getAttribute("href");
        return;
      }
      saveDraftToServer().then(function (saved) {
        if (saved) {
          window.location.href = link.getAttribute("href");
        }
      });
    });
  });

  textarea.addEventListener("input", function () {
    markDirty();
    persistDraft();
    scheduleRefresh();
  });

  window.addEventListener("beforeunload", function (event) {
    if (!isDirty) return;
    event.preventDefault();
    event.returnValue = "";
  });

  refreshFromTextArea();
}

document.addEventListener("DOMContentLoaded", bindWorkflowEditor);
