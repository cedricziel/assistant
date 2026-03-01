// -- App shell JS (loaded on every page via base.html) --

// -- Drawer toggle (tablet breakpoint) --------------------------------------

function toggleDrawer() {
  document.getElementById("drawer").classList.toggle("open");
  document.getElementById("drawerBackdrop").classList.toggle("open");
}

document.addEventListener("click", function (e) {
  if (e.target.closest(".hamburger")) {
    toggleDrawer();
    return;
  }
  if (e.target.closest(".drawer-close")) {
    toggleDrawer();
    return;
  }
  if (e.target.id === "drawerBackdrop") {
    toggleDrawer();
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
