# OpenClaw vs NanoClaw vs Our Assistant — Concept Comparison

> Research conducted February 2026 using a parallel agent team.

## Context: The "Claw" Ecosystem

In January–February 2026, OpenClaw (formerly Clawdbot → Moltbot) exploded to 196k+ GitHub stars — one of the fastest-growing OSS projects ever. NanoClaw emerged as a deliberate minimalist reaction to OpenClaw's security surface area. Our project predates or runs parallel to this, sharing some deep assumptions but with very different goals and implementation choices.

---

## 1. Scale & Philosophy

| Dimension | OpenClaw | NanoClaw | Our Assistant |
|---|---|---|---|
| **Scale** | ~350k+ lines, TypeScript | ~500 lines, TypeScript | Moderate, Rust workspace |
| **Language** | TypeScript | TypeScript | **Rust** |
| **Core stance** | "LLM as OS" — general-purpose framework | "Security through minimalism" | "Minimalist self-improving assistant" |
| **Audience** | Power users, developers, enterprises | Hackers who want to own every line | Personal use, self-hostable |
| **Auditability** | Requires trust in a large codebase | Readable in ~8 min | Medium — Rust types help |

**Key contrast:** OpenClaw and NanoClaw are both TypeScript. Our Rust choice buys us memory safety and compile-time correctness, but at the cost of ecosystem richness. NanoClaw's philosophy ("I couldn't trust software I couldn't read in an afternoon") is the closest philosophically to ours.

---

## 2. Orchestration Pattern

| Dimension | OpenClaw | NanoClaw | Our Assistant |
|---|---|---|---|
| **Loop type** | Pi agent loop: `intake → context assembly → LLM → tool execution → persistence` | Anthropic Agent SDK (delegates loop to SDK) | **ReAct loop**: `THOUGHT → ACTION → OBSERVATION → ANSWER` |
| **Concurrency model** | **Lane Queue** — serial per session, opt-in parallelism | Single process, polling model | Serial per `run_turn()` call |
| **Max iterations** | Not externally capped (auto-compacts context) | Uncapped (SDK handles) | **Configurable cap** (default: 10) |
| **Multi-agent** | Yes — multiple agents per Gateway with binding rules | Yes — Agent Swarms via SDK | No (single orchestrator) |
| **Proactive** | Yes — **heartbeat daemon** + cron jobs | Yes — scheduled tasks (cron/interval/once) | Partial — `ScheduleTaskHandler` exists but is nascent |

**Key contrast:** OpenClaw's Lane Queue is a sophisticated concurrency primitive. Our `run_turn()` is implicitly serial but doesn't manage queuing across concurrent callers. OpenClaw and NanoClaw are both proactive; our scheduler is an early concept. Neither external project implements explicit **iteration limits** — we are the only one with a hard `max_iterations` guard, which is both a safety feature and a limitation.

---

## 3. Skill / Tool System

| Dimension | OpenClaw | NanoClaw | Our Assistant |
|---|---|---|---|
| **Terminology** | **Tools** (raw function definitions) + **Skills** (behavioral overlays) | **Skills** (Claude Code transformation scripts) | **Skills** (SKILL.md declared, multi-tier) |
| **Definition format** | JSON schema + Markdown descriptions | `.claude/skills/<name>/SKILL.md` instruction files | `SKILL.md` with YAML frontmatter + Markdown body |
| **Execution model** | Sandbox (Docker), Host process, or Remote Nodes | Container subprocess per agent | **4 tiers: Prompt, Script, Wasm, Builtin** |
| **Discovery** | Gateway registry + ClawHub marketplace install | Skill files applied to fork at dev time | **Filesystem scan** at startup from known dirs |
| **Marketplace** | **ClawHub** — 700–5700+ community skills | Not applicable (fork-based model) | None (personal/local) |
| **Extension model** | Install from ClawHub, runtime plugins | Run `/add-telegram` in Claude Code → rewrites your fork's source | Register new `SkillHandler` in Rust |

**Key contrast:** The skill concepts are superficially similar (SKILL.md!) but diverge sharply in intent:

- **OpenClaw Skills** are runtime behavioral overlays — they modify how the model behaves without being a raw tool call.
- **NanoClaw Skills** are agentic source code transformations — they don't run at runtime at all; they teach Claude Code to rewrite your fork.
- **Our Skills** are the only ones with a proper **execution tier hierarchy** (Prompt → Script → Wasm → Builtin). We express capability as a spectrum, from pure LLM prompt augmentation to compiled Rust code. This is architecturally richer than OpenClaw/NanoClaw.

We share the SKILL.md format with NanoClaw — which is not surprising given NanoClaw is built on Claude Code skills. However our SKILL.md is a runtime artifact; theirs is a dev-time transformation tool.

---

## 4. Memory & Storage

| Dimension | OpenClaw | NanoClaw | Our Assistant |
|---|---|---|---|
| **Primary store** | Plain Markdown files (`AGENTS.md`, `MEMORY.md`, `SOUL.md`, etc.) + local SQLite | SQLite + per-group `CLAUDE.md` files | **SQLite exclusively** |
| **Memory retrieval** | Hybrid: 70% vector search + 30% BM25 keyword (RAG-lite, local SQLite embeddings) | Direct file read into context | SQL queries (no vector/semantic search) |
| **Conversation history** | JSONL file on disk (full), in-memory context (compacted) | SQLite messages table | **SQLite messages table** |
| **Self-improvement traces** | Not explicitly designed | Not present | **`distributed_traces` (OpenTelemetry spans)** — first-class concept |
| **Skill refinements** | Not present | Not present | **`RefinementsStore`** — proposed SKILL.md diffs with accept/reject workflow |
| **Memory isolation** | `MEMORY.md` never loads in group channels (privacy feature) | Per-group `CLAUDE.md` — fully isolated | Per-conversation isolation, no group concept |

**Key contrast:** OpenClaw's file-based memory is more human-readable and Git-friendly, but lacks structure. Our SQL-based model enables the `TraceStats` queries that power `self-analyze`. **We are the only project with a first-class self-improvement feedback loop** — execution traces, statistical analysis, and skill refinement proposals are built into the schema. Neither OpenClaw nor NanoClaw have this.

OpenClaw's hybrid vector+BM25 search is something we lack entirely — our `MemoryStore.search()` is presumably substring-based, which will degrade as memory grows.

---

## 5. LLM Integration

| Dimension | OpenClaw | NanoClaw | Our Assistant |
|---|---|---|---|
| **Providers** | 14+ (OpenAI, Anthropic, Gemini, DeepSeek, Ollama, vLLM, LM Studio, LiteLLM…) | **Claude only** (Anthropic Agent SDK) | **Ollama** (local, default), **Anthropic**, **OpenAI** (+ any OpenAI-compatible API) |
| **Tool calling** | Native (Pi agent framework handles) | Native (Anthropic Agent SDK handles) | **Dual-mode: native → ReAct text fallback** |
| **Local LLM support** | Yes (Ollama, vLLM, LM Studio) | No (Claude API required) | **Yes, Ollama-first** (OpenAI provider also works with local vLLM/LM Studio) |
| **Model agnosticism** | Fully agnostic via provider config | Locked to Claude | Provider-agnostic: Ollama, Anthropic, OpenAI (+ OpenAI-compatible endpoints) |
| **Fallback strategy** | Not described | Not needed | **Explicit `Auto` mode**: native tool calling → ReAct |

**Key contrast:** Our dual-mode LLM client (native tool calling → ReAct fallback) is the most sophisticated handling of model capability variance. OpenClaw delegates this to Pi (which presumably handles it). NanoClaw doesn't need it (Claude always supports native tool calling). We built this because Ollama models vary widely in tool-calling support — a pragmatic engineering choice. With Ollama, Anthropic, and OpenAI providers now implemented, we cover the most important local and cloud model sources. The OpenAI provider also supports any OpenAI-compatible endpoint (vLLM, LM Studio, OpenRouter, Azure), making the effective provider count much higher than three.

---

## 6. Security Model

| Dimension | OpenClaw | NanoClaw | Our Assistant |
|---|---|---|---|
| **Sandboxing** | Optional Docker containers for tool execution | **Mandatory OS-level container isolation** (Apple Container / Docker), one VM per container | **SafetyGate**: application-level per-interface rules |
| **Isolation level** | Application + optional Docker | **Hypervisor/kernel** (strongest possible) | Process-level (no container isolation) |
| **Per-interface security** | Separate agents per channel, per-agent credential isolation | Per-group container isolation | **Interface enum annotates context** (e.g., Signal turns auto-deny confirmation-required tools) |
| **Confirmation workflow** | Not described | Not present | **`ConfirmationCallback` trait** — interactive approval for mutating skills |
| **Supply chain risk** | High (ClawHub had ~12-20% malicious skills; VirusTotal partnership added) | Minimal (no marketplace; skills transform source at dev time) | None (no marketplace) |

**Key contrast:** NanoClaw has the strongest isolation model — hypervisor-enforced, not application-enforced. Our SafetyGate is pragmatic and correct for a single-user local assistant, but it could be bypassed by a compromised skill. NanoClaw's container model means even if a skill does something dangerous, it cannot escape the container. We have something they don't though: the **`ConfirmationCallback` trait** giving humans a veto on mutating operations at runtime.

---

## 7. Interfaces & Channels

| Dimension | OpenClaw | NanoClaw | Our Assistant |
|---|---|---|---|
| **Messaging platforms** | 12+ (WhatsApp, Telegram, Discord, Slack, Signal, iMessage, more) | WhatsApp (default), Telegram (via `/add-telegram` skill) | **CLI + Signal** (via presage) |
| **Interface abstraction** | Channel adapters normalize to common format | Per-group containers (each group is isolated) | **`Interface` enum** (`Cli`, `Signal`, `Mcp`) with context-aware gating |
| **Web UI** | Yes | No | No |
| **MCP server** | Yes (+ MCP Bridge for remote delegation) | Yes (MCP servers configurable in agent init) | **Yes — `crates/mcp-server`** |
| **API mode** | Via Gateway | No | Via MCP server |

**Key contrast:** OpenClaw's channel normalization is a proper adapter pattern that decouples agent logic from transport. Our `Interface` enum is simpler but achieves the same safety gating. Our Signal integration (via presage) is natively supported out-of-the-box, which matches OpenClaw's scope. NanoClaw is WhatsApp-first and relies on skills to add channels.

---

## 8. Self-Improvement & Proactive Behavior

| Concept | OpenClaw | NanoClaw | Our Assistant |
|---|---|---|---|
| **Execution tracing** | Not a core concept | Not present | **First-class: OpenTelemetry spans with duration, params, errors** |
| **Statistical analysis** | Not present | Not present | **`TraceStats`: success rate, avg duration, common errors** |
| **Skill refinement proposals** | Not present | Not present | **`RefinementsStore`: propose → review → accept/reject SKILL.md diffs** |
| **Proactive scheduling** | Heartbeat daemon (every 30 min default) + full cron | Task scheduler (cron/interval/once) | `ScheduleTaskHandler` (nascent) |
| **LLM self-modification** | LLM can write and execute code | Skills transform source at dev time | LLM proposes skill refinements via `self-analyze` |

**Key contrast: We are uniquely positioned on self-improvement.** Neither OpenClaw nor NanoClaw have a structured feedback loop from execution outcomes back to skill quality. Our `MirrorConfig.trace_enabled` + `self-analyze` skill + `RefinementsStore` is a genuine differentiator. However, we are behind on proactive scheduling — the heartbeat/cron pattern from both OpenClaw and NanoClaw is more mature than our nascent `ScheduleTaskHandler`.

---

## Summary: Alignment & Gaps

### Where we align with OpenClaw
- SQLite as the persistence backbone
- SKILL.md as the skill definition format
- MCP server exposure
- Local-first ethos (Ollama vs their Ollama support among many providers)
- Per-interface security gating

### Where we align with NanoClaw
- Minimalist philosophy ("self-improving personal assistant" vs "personal Claude assistant in 500 lines")
- Single-user focus
- Signal support (NanoClaw is WhatsApp, we are Signal)
- No marketplace — curated personal skills
- Skills as SKILL.md files

### Where we differ (gaps to consider)

| Gap | Them | Us | Priority |
|---|---|---|---|
| **Semantic memory search** | OpenClaw: hybrid vector+BM25 | Substring only | Medium |
| **Container isolation** | NanoClaw: hypervisor-enforced | Application-level SafetyGate | Low (single-user trusted) |
| **Proactive heartbeat** | Both: native daemon | Nascent scheduler | Medium |
| **Multi-LLM providers** | OpenClaw: 14+ | Ollama, Anthropic, OpenAI (+ compatible) | Low (narrowing) |
| **Context auto-compaction** | OpenClaw: auto-compacts to model window | Manual history management | Medium |
| **Lane queue** | OpenClaw: per-session serial queue | None explicit | Low (single-user) |

### Where we're ahead of both

1. **Execution tier system** — our 4-tier `SkillTier` (Prompt/Script/Wasm/Builtin) is architecturally richer than anything either project has.
2. **Self-improvement feedback loop** — distributed trace spans → `TraceStats` → `SelfAnalyzeHandler` → `RefinementsStore` is a first-class system neither has.
3. **Dual-mode LLM client** — native tool calling with explicit ReAct fallback handles model heterogeneity neither SDK-locked (NanoClaw) nor provider-agnostic (OpenClaw) approaches need to solve.
4. **Compile-time correctness** — Rust's type system catches entire classes of runtime errors that TypeScript cannot.

---

The clearest strategic insight: **OpenClaw chose breadth, NanoClaw chose security through minimalism, we chose self-improvement as the core differentiator.** That's a defensible and distinct position in this space.
