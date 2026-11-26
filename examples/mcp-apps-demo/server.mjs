// An MCP server with one MCP App, built on the official SDKs.
//
// `get-forecast` is offered to the model and renders the app page from
// `ui://demo/forecast.html`. `save-favorite` is offered to the app only, and
// is not read-only, so the host asks the user before the app may call it.
//
// Serves MCP over stdio. Run `npm run build` first to produce dist/app.html.

import { readFile } from "node:fs/promises";

import {
  RESOURCE_MIME_TYPE,
  registerAppResource,
  registerAppTool,
} from "@modelcontextprotocol/ext-apps/server";
import { McpServer } from "@modelcontextprotocol/server";
import { serveStdio } from "@modelcontextprotocol/server/stdio";
import { z } from "zod";

const APP_URI = "ui://demo/forecast.html";
const APP_HTML = new URL("./dist/app.html", import.meta.url);

const FORECASTS = {
  berlin: { temperature: 21, summary: "Sunny with a light breeze" },
  london: { temperature: 14, summary: "Light rain" },
};

const favorites = new Set();

function createServer() {
  const server = new McpServer({ name: "mcp-apps-demo", version: "1.0.0" });

  registerAppTool(
    server,
    "get-forecast",
    {
      title: "Get forecast",
      description: "Get today's weather forecast for a city.",
      inputSchema: z.object({ city: z.string().describe("City name") }),
      annotations: { readOnlyHint: true },
      _meta: { ui: { resourceUri: APP_URI } },
    },
    async ({ city }) => {
      const forecast = FORECASTS[city.toLowerCase()] ?? {
        temperature: 18,
        summary: "Partly cloudy",
      };
      return {
        content: [
          {
            type: "text",
            text: `${forecast.summary}, ${forecast.temperature}°C in ${city}`,
          },
        ],
        structuredContent: { city, ...forecast },
      };
    },
  );

  registerAppTool(
    server,
    "save-favorite",
    {
      title: "Save favorite city",
      description: "Save a city to the user's favorites.",
      inputSchema: z.object({ city: z.string() }),
      annotations: { readOnlyHint: false },
      _meta: { ui: { resourceUri: APP_URI, visibility: ["app"] } },
    },
    async ({ city }) => {
      favorites.add(city);
      return {
        content: [{ type: "text", text: `Saved ${city} to your favorites.` }],
        structuredContent: { favorites: [...favorites] },
      };
    },
  );

  registerAppResource(
    server,
    "Forecast",
    APP_URI,
    { description: "Shows a forecast and saves the city as a favorite." },
    async () => ({
      contents: [
        {
          uri: APP_URI,
          mimeType: RESOURCE_MIME_TYPE,
          text: await readFile(APP_HTML, "utf8"),
        },
      ],
    }),
  );

  return server;
}

serveStdio(createServer, {
  onerror: (error) => console.error("mcp-apps-demo:", error),
});
