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

document.addEventListener(
  "submit",
  function (e) {
    var form = e.target;
    if (!form || !form.getAttribute) return;
    var message = form.getAttribute("data-confirm");
    if (!message) return;
    if (!window.confirm(message)) {
      e.preventDefault();
    }
  },
  true,
);

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
