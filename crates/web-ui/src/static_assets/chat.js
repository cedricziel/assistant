// -- Chat-specific JS (loaded only on chat pages) --

// -- Enter-to-submit (Shift+Enter for newline) ------------------------------

document.addEventListener("keydown", function (e) {
  if (!e.target.matches(".chat-input-field")) return;
  if (e.key === "Enter" && !e.shiftKey) {
    e.preventDefault();
    e.target.form.requestSubmit();
  }
});

// -- Textarea auto-resize ---------------------------------------------------

document.addEventListener("input", function (e) {
  if (!e.target.matches(".chat-input-field")) return;
  e.target.style.height = "auto";
  e.target.style.height = Math.min(e.target.scrollHeight, 160) + "px";
});

// -- Scroll messages to bottom ----------------------------------------------

function scrollMessagesToBottom() {
  var msgs = document.getElementById("messages");
  if (msgs) msgs.scrollTop = msgs.scrollHeight;
}

document.addEventListener("DOMContentLoaded", scrollMessagesToBottom);

document.addEventListener("htmx:afterSwap", function (e) {
  if (e.detail.target.id === "messages") {
    scrollMessagesToBottom();
  }
  // When a conversation is loaded into the chat panel, activate mobile layout
  if (e.detail.target.id === "chat-panel") {
    var layout = document.getElementById("chat-layout");
    if (layout) layout.classList.add("chat-active");
    scrollMessagesToBottom();
    // Mark the correct sidebar item as active
    var id = window.location.pathname.split("/").pop();
    document.querySelectorAll(".conv-item").forEach(function (el) {
      el.classList.remove("active");
    });
    var activeLink = document.querySelector(
      '.conv-item[href="/chat/' + id + '"]',
    );
    if (activeLink) activeLink.classList.add("active");
  }
});

// -- Streaming UX -----------------------------------------------------------

// Called after the send request returns (user bubble + streaming skeleton)
function chatStreamStarted() {
  var btn = document.getElementById("btn-send");
  var field = document.querySelector(".chat-input-field");
  if (btn) {
    btn.disabled = true;
    btn.textContent = "Thinking...";
  }
  if (field) field.disabled = true;
}

// Re-enable the form when the streaming skeleton is replaced by the final
// message (the "done" SSE event triggers an outerHTML swap that removes
// #streaming-msg from the DOM).
function chatStreamEnded() {
  var btn = document.getElementById("btn-send");
  var field = document.querySelector(".chat-input-field");
  if (btn) {
    btn.disabled = false;
    btn.textContent = "Send";
  }
  if (field) {
    field.disabled = false;
    field.focus();
  }
}

// Detect when the streaming skeleton is removed (done event fired).
document.addEventListener("DOMContentLoaded", function () {
  var msgs = document.getElementById("messages");
  if (!msgs) return;

  var observer = new MutationObserver(function () {
    if (!document.getElementById("streaming-msg")) {
      chatStreamEnded();
    }
  });
  observer.observe(msgs, { childList: true, subtree: true });
});

// Auto-scroll during streaming (token events).
document.addEventListener("htmx:sseMessage", function () {
  scrollMessagesToBottom();
});

// Wire up the after-request handler for chat forms.
document.addEventListener("htmx:afterRequest", function (e) {
  var form = e.detail.elt;
  if (!form || !form.matches || !form.matches("#chat-form")) return;
  form.reset();
  chatStreamStarted();
});
