// Keeps the docs sidebar where you left it when you open another page (sessionStorage).
// Without a saved position, or if the current page's link would be out of view, it brings
// that link into view instead. The page works the same without it.
(function () {
  var side = document.querySelector(".side");
  if (!side) return;
  var key = "rusttd6-docs-side";
  try {
    var saved = sessionStorage.getItem(key);
    if (saved !== null) side.scrollTop = +saved;
  } catch (e) {}
  var current = side.querySelector('[aria-current="page"]');
  if (current) {
    var s = side.getBoundingClientRect(), c = current.getBoundingClientRect();
    if (c.top < s.top || c.bottom > s.bottom) current.scrollIntoView({ block: "center" });
  }
  addEventListener("pagehide", function () {
    try { sessionStorage.setItem(key, side.scrollTop); } catch (e) {}
  });
})();
