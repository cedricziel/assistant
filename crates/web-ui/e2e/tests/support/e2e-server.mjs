// Starts the e2e server: `webui serve` with HOME set to $ASSISTANT_E2E_HOME
// and, once it is up, the `orchestrator run` that executes its turns. The
// HOME lists the examples/mcp-apps-demo MCP server under [mcp] and uses the
// scripted model from mock-ollama.mjs. Stopping this process stops both.
//
// Usage: ASSISTANT_E2E_HOME=<dir> node e2e-server.mjs <mock LLM port> <assistant command...>
//
// The directory comes through the environment so the shell that Playwright
// runs this command in never parses it. Its config files are overwritten;
// nothing else in it is removed.

import { spawn } from "node:child_process";
import { existsSync, mkdirSync, writeFileSync } from "node:fs";
import * as path from "node:path";
import { fileURLToPath } from "node:url";

const home = process.env.ASSISTANT_E2E_HOME;
const [llmPort, ...assistant] = process.argv.slice(2);
if (!home || !llmPort || assistant.length === 0) {
  throw new Error(
    "usage: ASSISTANT_E2E_HOME=<dir> e2e-server.mjs <mock LLM port> <assistant command...>",
  );
}

const demo = path.resolve(
  path.dirname(fileURLToPath(import.meta.url)),
  "../../../../../examples/mcp-apps-demo",
);
if (!existsSync(path.join(demo, "dist", "app.html"))) {
  console.error(
    `The MCP Apps demo is not built. Run: (cd ${demo} && npm ci && npm run build)`,
  );
  process.exit(1);
}

const config = path.join(home, ".assistant");
mkdirSync(path.join(config, "orgs", "default"), { recursive: true });
writeFileSync(
  path.join(config, "server.toml"),
  `[[mcp.servers]]
name = "demo"
command = [${JSON.stringify(process.execPath)}, ${JSON.stringify(path.join(demo, "server.mjs"))}]
trust = "trust"
`,
);
writeFileSync(
  path.join(config, "orgs", "default", "org.toml"),
  `[llm]
provider = "ollama"
model = "mock"
base_url = "http://127.0.0.1:${llmPort}"

[memory]
enabled = false

[permissions]
allow = ["mcp__demo__get-forecast"]
`,
);

const children = [];
const run = (args) => {
  const child = spawn(assistant[0], [...assistant.slice(1), ...args], {
    env: { ...process.env, HOME: home },
    stdio: "inherit",
  });
  child.on("exit", (code) => {
    if (!stopping) {
      console.error(`assistant ${args.join(" ")} exited with ${code}`);
      stop(1);
    }
  });
  children.push(child);
};

let stopping = false;
const stop = (code = 0) => {
  stopping = true;
  for (const child of children) child.kill("SIGTERM");
  process.exit(code);
};
process.on("SIGINT", () => stop());
process.on("SIGTERM", () => stop());

run([
  "webui",
  "serve",
  "--auth-token",
  "test-token",
  "--listen",
  "127.0.0.1:8787",
]);
for (;;) {
  const up = await fetch("http://127.0.0.1:8787/login").then(
    (r) => r.ok,
    () => false,
  );
  if (up) break;
  await new Promise((resolve) => setTimeout(resolve, 500));
}
run(["orchestrator", "run", "--no-repl"]);
