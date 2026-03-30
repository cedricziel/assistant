import { Controller } from "@hotwired/stimulus";

/**
 * Manages the right-hand chat panel: conversation history, compose form, SSE streaming.
 *
 * Targets:
 *   emptyState   — shown when no conversation is open
 *   activePanel  — the conversation area (header + messages + form)
 *   title        — <h2> for the active conversation title
 *   mobileTitle  — title span in the mobile header
 *   messages     — scrollable message list container
 *   messageInput — the compose <textarea>
 *   sendButton   — the Send <button>
 *
 * Values:
 *   conversationId (string) — UUID of the open conversation
 *
 * Dispatches (bubbles to window):
 *   message-panel:deleted   — after a conversation is deleted
 *     detail: { id }
 *   message-panel:responded — after the assistant finishes a turn
 *     detail: { id }
 *
 * Listens (via data-action on this element):
 *   conversation-list:selected@window → open(event)
 */
export default class MessagePanelController extends Controller {
  static targets = [
    "emptyState",
    "activePanel",
    "title",
    "mobileTitle",
    "messages",
    "messageInput",
    "sendButton",
  ];
  static values = { conversationId: String };

  // ---------------------------------------------------------------------------
  // Lifecycle
  // ---------------------------------------------------------------------------

  connect() {
    // Deep-link: if the page loaded at /chat/{id}, open that conversation.
    const id = this._idFromUrl();
    if (id) this.open({ detail: { id } });

    this._popHandler = (event) => {
      const id = event.state?.id || this._idFromUrl();
      if (id) this.open({ detail: { id } });
      else this._showEmpty();
    };
    window.addEventListener("popstate", this._popHandler);
  }

  disconnect() {
    window.removeEventListener("popstate", this._popHandler);
  }

  // ---------------------------------------------------------------------------
  // Cross-controller event handlers
  // ---------------------------------------------------------------------------

  /** Open a conversation. Triggered by conversation-list:selected@window. */
  async open(event) {
    const { id } = event.detail;
    if (!id) return;
    this.conversationIdValue = id;

    try {
      const res = await fetch(`/api/conversations/${id}`);
      if (!res.ok) return;
      const conv = await res.json();
      this._render(conv);
    } catch (e) {
      console.error("Failed to load conversation:", e);
    }
  }

  // ---------------------------------------------------------------------------
  // Actions
  // ---------------------------------------------------------------------------

  /** Send a user message and stream the assistant response. */
  async sendMessage(event) {
    event.preventDefault();
    const input = this.messageInputTarget;
    const message = input.value.trim();
    if (!message || !this.conversationIdValue) return;

    input.value = "";
    input.style.height = "auto";
    this._setInputEnabled(false);

    this._appendMessage("user", message);
    const bubble = this._appendStreamingBubble();

    try {
      const res = await fetch(
        `/api/conversations/${this.conversationIdValue}/messages`,
        {
          method: "POST",
          headers: { "Content-Type": "application/json" },
          body: JSON.stringify({ message }),
        },
      );
      if (!res.ok || !res.body) return;

      const contentEl = bubble.querySelector(".msg-content");
      await this._parseSSE(res.body, {
        token: (data) => {
          contentEl.textContent += data;
          this._scrollBottom();
        },
        done: (data) => {
          try {
            contentEl.textContent = JSON.parse(data).content;
          } catch {
            contentEl.textContent = data;
          }
          this._scrollBottom();
        },
      });

      this.dispatch("responded", {
        detail: { id: this.conversationIdValue },
        bubbles: true,
      });
    } catch (e) {
      console.error("Failed to send message:", e);
    } finally {
      this._setInputEnabled(true);
    }
  }

  /** Delete the current conversation. */
  async deleteConversation(event) {
    event.preventDefault();
    if (!this.conversationIdValue) return;
    if (!confirm("Delete this conversation?")) return;

    const id = this.conversationIdValue;
    try {
      await fetch(`/api/conversations/${id}`, { method: "DELETE" });
    } catch (e) {
      console.error("Failed to delete conversation:", e);
      return;
    }

    this.conversationIdValue = "";
    history.pushState({}, "", "/chat");
    this.dispatch("deleted", { detail: { id }, bubbles: true });
    this._showEmpty();
  }

  /** Mobile back button — return to the conversation list. */
  goBack(event) {
    event.preventDefault();
    this.conversationIdValue = "";
    history.pushState({}, "", "/chat");
    this._showEmpty();
  }

  /** Allow Shift+Enter for newline; plain Enter submits. */
  handleKeydown(event) {
    if (event.key === "Enter" && !event.shiftKey) {
      event.preventDefault();
      event.target.form.requestSubmit();
    }
  }

  /** Auto-resize the textarea. */
  handleInput(event) {
    const el = event.target;
    el.style.height = "auto";
    el.style.height = Math.min(el.scrollHeight, 160) + "px";
  }

  // ---------------------------------------------------------------------------
  // Private
  // ---------------------------------------------------------------------------

  _render(conv) {
    this.titleTarget.textContent = conv.title;
    if (this.hasMobileTitleTarget) {
      this.mobileTitleTarget.textContent = conv.title;
    }

    const container = this.messagesTarget;
    container.innerHTML = "";
    for (const msg of conv.messages) {
      container.appendChild(
        this._buildMessage(msg.role, msg.content, new Date(msg.created_at)),
      );
    }

    this.emptyStateTarget.style.display = "none";
    this.activePanelTarget.style.display = "flex";
    this._chatLayout()?.classList.add("chat-active");
    this._scrollBottom();
  }

  _showEmpty() {
    this.emptyStateTarget.style.display = "";
    this.activePanelTarget.style.display = "none";
    this._chatLayout()?.classList.remove("chat-active");
  }

  _appendMessage(role, content) {
    this.messagesTarget.appendChild(
      this._buildMessage(role, content, new Date()),
    );
    this._scrollBottom();
  }

  _appendStreamingBubble() {
    const el = document.createElement("div");
    el.className = "message msg-assistant";
    el.innerHTML = `
      <div class="msg-header">
        <span class="msg-role">Assistant</span>
        <span class="msg-time">${this._formatTime(new Date())}</span>
      </div>
      <div class="msg-content"></div>`;
    this.messagesTarget.appendChild(el);
    this._scrollBottom();
    return el;
  }

  _buildMessage(role, content, date) {
    const el = document.createElement("div");
    el.className = `message ${role === "user" ? "msg-user" : "msg-assistant"}`;
    const header = document.createElement("div");
    header.className = "msg-header";
    header.innerHTML = `<span class="msg-role">${role === "user" ? "You" : "Assistant"}</span><span class="msg-time">${this._formatTime(date)}</span>`;
    const body = document.createElement("div");
    body.className = "msg-content";
    body.textContent = content;
    el.append(header, body);
    return el;
  }

  async _parseSSE(body, handlers) {
    const reader = body.getReader();
    const decoder = new TextDecoder();
    let buffer = "";
    let currentEvent = "";

    while (true) {
      const { value, done } = await reader.read();
      if (done) break;
      buffer += decoder.decode(value, { stream: true });
      let nl;
      while ((nl = buffer.indexOf("\n")) !== -1) {
        const line = buffer.slice(0, nl);
        buffer = buffer.slice(nl + 1);
        if (line.startsWith("event: ")) {
          currentEvent = line.slice(7).trim();
        } else if (line.startsWith("data: ")) {
          if (currentEvent && handlers[currentEvent]) {
            handlers[currentEvent](line.slice(6));
          }
          currentEvent = "";
        }
      }
    }
  }

  _idFromUrl() {
    const m = window.location.pathname.match(/^\/chat\/([0-9a-f-]{36})$/i);
    return m ? m[1] : null;
  }

  _chatLayout() {
    return this.element.closest(".chat-layout");
  }

  _scrollBottom() {
    const el = this.messagesTarget;
    el.scrollTop = el.scrollHeight;
  }

  _formatTime(d) {
    return d.toLocaleTimeString([], { hour: "numeric", minute: "2-digit" });
  }

  _setInputEnabled(enabled) {
    this.sendButtonTarget.disabled = !enabled;
    this.sendButtonTarget.textContent = enabled ? "Send" : "Thinking...";
    this.messageInputTarget.disabled = !enabled;
    if (enabled) this.messageInputTarget.focus();
  }
}
