// A scripted stand-in for Ollama's /api/chat, so e2e turns run without a
// model. A user question about the weather gets a call to the demo MCP
// server's get-forecast tool; the tool's result gets a one-line answer; any
// other request (such as the conversation title) gets a short title. A
// message containing "hold this turn" gets no answer for a minute, so a client
// can cancel the turn while it runs.
//
// Usage: node mock-ollama.mjs <port>

import { createServer } from "node:http";

const port = Number(process.argv[2] ?? 18788);
const FORECAST_TOOL = "mcp__demo__get-forecast";
const HOLD = /hold this turn/i;
const HOLD_MS = 60_000;

function reply(body) {
  const messages = body.messages ?? [];
  const last = messages[messages.length - 1] ?? {};
  const offersForecast = (body.tools ?? []).some(
    (tool) => tool.function?.name === FORECAST_TOOL,
  );
  if (last.role === "tool") {
    return { role: "assistant", content: "It is sunny and 21°C in Berlin." };
  }
  if (offersForecast && /weather/i.test(last.content ?? "")) {
    return {
      role: "assistant",
      content: "",
      tool_calls: [
        { function: { name: FORECAST_TOOL, arguments: { city: "Berlin" } } },
      ],
    };
  }
  return { role: "assistant", content: "Weather in Berlin" };
}

createServer((req, res) => {
  let raw = "";
  req.on("data", (chunk) => (raw += chunk));
  req.on("end", () => {
    if (req.url === "/api/chat" && req.method === "POST") {
      const body = JSON.parse(raw || "{}");
      const last = (body.messages ?? []).at(-1) ?? {};
      if (HOLD.test(last.content ?? "")) {
        setTimeout(() => res.writeHead(503).end(), HOLD_MS);
        return;
      }
      const chunk = {
        model: body.model,
        message: reply(body),
        done: true,
        done_reason: "stop",
        prompt_eval_count: 1,
        eval_count: 1,
      };
      res.writeHead(200, {
        "content-type": body.stream
          ? "application/x-ndjson"
          : "application/json",
      });
      res.end(JSON.stringify(chunk) + "\n");
    } else if (req.url === "/api/embed" && req.method === "POST") {
      const body = JSON.parse(raw || "{}");
      const inputs = [body.input ?? ""].flat();
      res.writeHead(200, { "content-type": "application/json" });
      res.end(
        JSON.stringify({ embeddings: inputs.map(() => Array(768).fill(0)) }),
      );
    } else if (req.url === "/api/tags") {
      res.writeHead(200, { "content-type": "application/json" });
      res.end(JSON.stringify({ models: [{ name: "mock" }] }));
    } else {
      res.writeHead(404).end();
    }
  });
}).listen(port, "127.0.0.1");
