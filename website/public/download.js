// Download, then show how to install: the button still starts the download (its href), and a
// moment later the page scrolls to the install steps.
(function () {
  var install = document.getElementById("install");
  if (!install) return;
  var calm = window.matchMedia && window.matchMedia("(prefers-reduced-motion: reduce)").matches;
  document.querySelectorAll("a.download").forEach(function (a) {
    a.addEventListener("click", function () {
      setTimeout(function () {
        install.scrollIntoView({ behavior: calm ? "auto" : "smooth", block: "start" });
      }, 400);
    });
  });
})();
