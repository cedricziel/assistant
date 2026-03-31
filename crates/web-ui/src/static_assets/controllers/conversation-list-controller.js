import { Controller } from "@hotwired/stimulus";

/**
 * Manages the conversation sidebar: loading, search, create, highlight active.
 *
 * Targets:
 *   list        — the <div class="conv-items"> container
 *   searchInput — the search <input>
 *
 * Values:
 *   activeId (string) — UUID of the currently highlighted conversation
 *
 * Dispatches (bubbles to window):
 *   conversation-list:selected — when an item is clicked or created
 *     detail: { id }
 *
 * Listens (via data-action on this element):
 *   message-panel:deleted  → removeDeleted(event)
 *   message-panel:responded → bumpToTop(event)
 */
export default class ConversationListController extends Controller {
  static targets = ["list", "searchInput"];
  static values = { activeId: String };

  /** Full list loaded from the API. */
  _conversations = [];
  _searchQuery = "";

  // ---------------------------------------------------------------------------
  // Lifecycle
  // ---------------------------------------------------------------------------

  connect() {
    this._load();

    this._popHandler = () => this._syncFromUrl();
    window.addEventListener("popstate", this._popHandler);
  }

  disconnect() {
    window.removeEventListener("popstate", this._popHandler);
  }

  // ---------------------------------------------------------------------------
  // Actions
  // ---------------------------------------------------------------------------

  /** Create a new conversation and notify the message panel to open it. */
  async newConversation(event) {
    event.preventDefault();
    try {
      const res = await fetch("/api/conversations", {
        method: "POST",
        headers: { "Content-Type": "application/json" },
        body: JSON.stringify({ title: "New Chat" }),
      });
      if (!res.ok) return;
      const conv = await res.json();
      this._conversations.unshift(conv);
      this.activeIdValue = conv.id;
      this._renderList();
      history.pushState({ id: conv.id }, "", `/chat/${conv.id}`);
      this.dispatch("selected", { detail: { id: conv.id }, bubbles: true });
    } catch (e) {
      console.error("Failed to create conversation:", e);
    }
  }

  /** Filter displayed conversations as the user types. */
  search(event) {
    this._searchQuery = event.target.value.toLowerCase();
    this._renderList();
  }

  /** Select a conversation item from the rendered list. */
  clickItem(event) {
    event.preventDefault();
    const { id } = event.currentTarget.dataset;
    if (!id || id === this.activeIdValue) return;
    this.activeIdValue = id;
    this._renderList();
    history.pushState({ id }, "", `/chat/${id}`);
    this.dispatch("selected", { detail: { id }, bubbles: true });
  }

  // ---------------------------------------------------------------------------
  // Cross-controller event handlers (wired via data-action in the template)
  // ---------------------------------------------------------------------------

  /** Called when message-panel:deleted fires — remove the item from the list. */
  removeDeleted(event) {
    const { id } = event.detail;
    this._conversations = this._conversations.filter((c) => c.id !== id);
    if (this.activeIdValue === id) this.activeIdValue = "";
    this._renderList();
  }

  /** Called when message-panel:responded fires — move conversation to the top. */
  bumpToTop(event) {
    const { id } = event.detail;
    const idx = this._conversations.findIndex((c) => c.id === id);
    if (idx > 0) {
      const [conv] = this._conversations.splice(idx, 1);
      conv.updated_at = new Date().toISOString();
      this._conversations.unshift(conv);
      this._renderList();
    }
  }

  // ---------------------------------------------------------------------------
  // Private
  // ---------------------------------------------------------------------------

  async _load() {
    try {
      const res = await fetch("/api/conversations");
      if (!res.ok) return;
      this._conversations = await res.json();
      this._syncFromUrl();
    } catch (e) {
      console.error("Failed to load conversations:", e);
    }
  }

  _syncFromUrl() {
    const id = this._idFromUrl();
    this.activeIdValue = id ?? "";
    this._renderList();
    if (id) {
      this.dispatch("selected", { detail: { id }, bubbles: true });
    }
  }

  _renderList() {
    const q = this._searchQuery;
    const items = q
      ? this._conversations.filter((c) => c.title.toLowerCase().includes(q))
      : this._conversations;

    if (items.length === 0) {
      this.listTarget.innerHTML =
        '<div style="padding:20px;text-align:center;color:#5a7396;font-size:0.85rem">No conversations yet</div>';
      return;
    }

    this.listTarget.innerHTML = items
      .map((c) => {
        const active = c.id === this.activeIdValue;
        return `<a class="conv-item${active ? " active" : ""}" href="/chat/${c.id}" data-id="${c.id}" data-action="click->conversation-list#clickItem">
          <div class="conv-item-title">${active ? '<span class="conv-item-dot"></span>' : ""}${this._esc(c.title)}</div>
          <div class="conv-item-meta">${this._timeAgo(c.updated_at)}</div>
        </a>`;
      })
      .join("");
  }

  _idFromUrl() {
    const m = window.location.pathname.match(/^\/chat\/([0-9a-f-]{36})$/i);
    return m ? m[1] : null;
  }

  _esc(str) {
    return String(str)
      .replace(/&/g, "&amp;")
      .replace(/</g, "&lt;")
      .replace(/>/g, "&gt;")
      .replace(/"/g, "&quot;");
  }

  _timeAgo(iso) {
    const secs = Math.floor((Date.now() - new Date(iso).getTime()) / 1000);
    if (secs < 60) return "just now";
    const mins = Math.floor(secs / 60);
    if (mins < 60) return mins === 1 ? "1 min ago" : `${mins} min ago`;
    const hrs = Math.floor(mins / 60);
    if (hrs < 24) return hrs === 1 ? "1 hr ago" : `${hrs} hr ago`;
    const days = Math.floor(hrs / 24);
    if (days < 7) return days === 1 ? "Yesterday" : `${days} days ago`;
    return new Date(iso).toLocaleDateString([], {
      month: "short",
      day: "numeric",
    });
  }
}
