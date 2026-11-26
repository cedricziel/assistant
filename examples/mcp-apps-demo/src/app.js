// The app page: renders the tool input and result the host sends, and calls
// the app-only `save-favorite` tool through the host.

import { App } from "@modelcontextprotocol/ext-apps";

const $ = (id) => document.getElementById(id);
const app = new App({ name: "mcp-apps-demo", version: "1.0.0" });
let city;

app.ontoolinput = ({ arguments: args }) => {
  city = args?.city;
  $("city").textContent = city ?? "";
};

app.ontoolresult = (result) => {
  const forecast = result.structuredContent ?? {};
  $("temp").textContent = `${forecast.temperature}°C`;
  $("summary").textContent = forecast.summary ?? "";
  $("save").disabled = !city;
};

$("save").addEventListener("click", async () => {
  $("save").disabled = true;
  try {
    const result = await app.callServerTool({
      name: "save-favorite",
      arguments: { city },
    });
    $("status").textContent = result.content?.[0]?.text ?? "Saved.";
  } catch (error) {
    $("status").textContent = `Not saved: ${error.message}`;
    $("save").disabled = false;
  }
});

await app.connect();
