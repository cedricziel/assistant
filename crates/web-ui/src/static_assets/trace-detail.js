// -- Trace detail: span attribute viewer --

(function () {
  var rows = document.querySelectorAll(".wf-row");
  var panel = document.getElementById("span-attrs-content");
  if (!panel) return;

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
        panel.textContent = "Could not parse attributes.";
        return;
      }

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

      if (data.span_id) addRow("span_id", data.span_id);
      if (data.name) addRow("name", data.name);
      if (data.tool_name) addRow("tool_name", data.tool_name);
      if (data.status) addRow("status", data.status);
      if (data.duration_ms !== undefined)
        addRow("duration_ms", data.duration_ms + " ms");
      if (data.observation) addRow("observation", data.observation);
      if (data.error) addRow("error", data.error);
      if (data.attributes && typeof data.attributes === "object") {
        Object.keys(data.attributes).forEach(function (k) {
          addRow(k, data.attributes[k]);
        });
      }

      panel.textContent = "";
      panel.appendChild(tbl);
    });
  });
})();
