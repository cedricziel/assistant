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
    tr.innerHTML =
      "<td>" +
      node.id +
      "</td><td>" +
      node.kind +
      "</td><td><code>" +
      JSON.stringify(node.config || {}) +
      "</code></td>";
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

      graph.edges.push({
        from: from,
        to: to,
        on: onInput.value.trim() || null,
      });
      writeGraph(textarea, graph, errorBox);
      refreshFromTextArea();
    });
  }

  textarea.addEventListener("input", function () {
    refreshFromTextArea();
  });

  refreshFromTextArea();
}

document.addEventListener("DOMContentLoaded", bindWorkflowEditor);
