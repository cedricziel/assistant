// -- App shell JS (loaded on every page via base.html) --

// -- Drawer toggle (tablet breakpoint) --------------------------------------

var drawer = document.getElementById("drawer");
var drawerBackdrop = document.getElementById("drawerBackdrop");
var drawerToggle = document.getElementById("drawerToggle");
var lastFocusedElement = null;

function drawerFocusableElements() {
  if (!drawer) return [];
  return Array.prototype.slice.call(
    drawer.querySelectorAll(
      'a[href], button:not([disabled]), input:not([disabled]), select:not([disabled]), textarea:not([disabled]), [tabindex]:not([tabindex="-1"])',
    ),
  );
}

function closeDrawer(restoreFocus) {
  if (!drawer || !drawerBackdrop) return;
  drawer.classList.remove("open");
  drawerBackdrop.classList.remove("open");
  drawer.setAttribute("aria-hidden", "true");
  if (drawerToggle) drawerToggle.setAttribute("aria-expanded", "false");
  if (restoreFocus && lastFocusedElement && lastFocusedElement.focus) {
    lastFocusedElement.focus();
  }
}

function openDrawer() {
  if (!drawer || !drawerBackdrop) return;
  lastFocusedElement = document.activeElement;
  drawer.classList.add("open");
  drawerBackdrop.classList.add("open");
  drawer.setAttribute("aria-hidden", "false");
  if (drawerToggle) drawerToggle.setAttribute("aria-expanded", "true");
  var focusables = drawerFocusableElements();
  if (focusables.length > 0) focusables[0].focus();
}

function toggleDrawer() {
  if (!drawer || !drawerBackdrop) return;
  if (drawer.classList.contains("open")) {
    closeDrawer(true);
  } else {
    openDrawer();
  }
}

function trapDrawerFocus(event) {
  if (event.key !== "Tab" || !drawer || !drawer.classList.contains("open")) {
    return;
  }
  var focusables = drawerFocusableElements();
  if (focusables.length === 0) return;
  var first = focusables[0];
  var last = focusables[focusables.length - 1];

  if (event.shiftKey && document.activeElement === first) {
    event.preventDefault();
    last.focus();
    return;
  }

  if (!event.shiftKey && document.activeElement === last) {
    event.preventDefault();
    first.focus();
  }
}

document.addEventListener("click", function (e) {
  if (e.target.closest(".hamburger")) {
    toggleDrawer();
    return;
  }
  if (e.target.closest(".drawer-close")) {
    closeDrawer(true);
    return;
  }
  if (e.target.id === "drawerBackdrop") {
    closeDrawer(true);
    return;
  }
  if (e.target.closest(".drawer-nav-item")) {
    closeDrawer(false);
    return;
  }

  var revealBtn = e.target.closest("[data-reveal-webhook-secrets]");
  if (!revealBtn) return;
  var tokenTarget = document.getElementById(
    revealBtn.getAttribute("data-token-target"),
  );
  var urlTarget = document.getElementById(
    revealBtn.getAttribute("data-url-target"),
  );
  if (!tokenTarget || !urlTarget) return;

  var isRevealed = revealBtn.getAttribute("data-revealed") === "true";
  if (isRevealed) {
    tokenTarget.textContent = revealBtn.getAttribute("data-token-masked") || "";
    urlTarget.textContent = revealBtn.getAttribute("data-url-masked") || "";
    revealBtn.textContent = "Reveal";
    revealBtn.setAttribute("data-revealed", "false");
    return;
  }

  tokenTarget.textContent = revealBtn.getAttribute("data-token") || "";
  urlTarget.textContent = revealBtn.getAttribute("data-url") || "";
  revealBtn.textContent = "Hide";
  revealBtn.setAttribute("data-revealed", "true");
});

// -- Clickable table rows ([data-href]) -------------------------------------

document.addEventListener("click", function (e) {
  var row = e.target.closest("[data-href]");
  if (!row) return;
  // Don't intercept clicks on links inside the row
  if (e.target.closest("a")) return;
  window.location = row.getAttribute("data-href");
});

document.addEventListener("keydown", function (e) {
  if (e.key === "Escape" && drawer && drawer.classList.contains("open")) {
    closeDrawer(true);
    return;
  }
  trapDrawerFocus(e);

  if (e.key !== "Enter") return;
  var row = e.target.closest("[data-href]");
  if (row) window.location = row.getAttribute("data-href");
});

// -- Service Worker registration --------------------------------------------

if ("serviceWorker" in navigator) {
  window.addEventListener("load", function () {
    navigator.serviceWorker
      .register("/sw.js", { scope: "/" })
      .then(function (reg) {
        setInterval(function () {
          reg.update();
        }, 60000);
      });
  });
}
