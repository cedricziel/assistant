// -- Trace detail: span attribute viewer --

(function () {
  var rows = document.querySelectorAll(".wf-row");
  var spanPanel = document.getElementById("span-attrs-content");
  var resourcePanel = document.getElementById("span-resource-content");
  if (!spanPanel || !resourcePanel) return;

  function isResourceKey(key) {
    return (
      key.indexOf("service.") === 0 ||
      key.indexOf("host.") === 0 ||
      key.indexOf("os.") === 0 ||
      key.indexOf("process.") === 0 ||
      key.indexOf("telemetry.") === 0
    );
  }

  function renderTable(panel, rowsData) {
    var tbl = document.createElement("table");
    tbl.className = "attr-table";

    function addRow(k, v) {
      var tr = document.createElement("tr");
      var tdK = document.createElement("td");
      tdK.className = "attr-k";
      tdK.textContent = k;
      var tdV = document.createElement("td");
      tdV.className = "attr-v";
      tdV.textContent = String(v);
      tr.appendChild(tdK);
      tr.appendChild(tdV);
      tbl.appendChild(tr);
    }

    if (rowsData.length === 0) {
      panel.textContent = "No attributes available.";
      return;
    }

    rowsData.forEach(function (entry) {
      addRow(entry[0], entry[1]);
    });

    panel.textContent = "";
    panel.appendChild(tbl);
  }

  rows.forEach(function (row) {
    row.addEventListener("keydown", function (e) {
      if (e.key === "Enter" || e.key === " ") {
        e.preventDefault();
        row.click();
      }
    });

    row.addEventListener("click", function () {
      rows.forEach(function (r) {
        r.classList.remove("selected");
      });
      row.classList.add("selected");

      var raw = row.getAttribute("data-attrs");
      var data;
      try {
        data = JSON.parse(raw);
      } catch (e) {
        spanPanel.textContent = "Could not parse attributes.";
        resourcePanel.textContent = "Could not parse attributes.";
        return;
      }

      var spanRows = [];
      var resourceRows = [];

      if (data.service_name) {
        resourceRows.push(["service.name", data.service_name]);
      }
      if (data.span_id) spanRows.push(["span_id", data.span_id]);
      if (data.name) spanRows.push(["name", data.name]);
      if (data.tool_name) spanRows.push(["tool_name", data.tool_name]);
      if (data.status) spanRows.push(["status", data.status]);
      if (data.duration_ms !== undefined)
        spanRows.push(["duration_ms", data.duration_ms + " ms"]);
      if (data.observation) spanRows.push(["observation", data.observation]);
      if (data.error) spanRows.push(["error", data.error]);

      if (data.attributes && typeof data.attributes === "object") {
        Object.keys(data.attributes)
          .sort()
          .forEach(function (k) {
            if (isResourceKey(k)) {
              resourceRows.push([k, data.attributes[k]]);
            } else {
              spanRows.push([k, data.attributes[k]]);
            }
          });
      }

      renderTable(spanPanel, spanRows);
      renderTable(resourcePanel, resourceRows);
    });
  });
})();
