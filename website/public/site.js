// Small touches on the home page. Without this script the page works the same, minus these.
(function () {
// The header's link for the section being read is marked (aria-current), in both menus.
(function () {
  var sections = Array.prototype.filter.call(document.querySelectorAll("main section[id]"), function (s) {
    return document.querySelector('header a[href="#' + s.id + '"]');
  });
  var current = null, queued = false;
  function update() {
    queued = false;
    var line = window.innerHeight * 0.35, found = null;
    sections.forEach(function (s) { if (s.getBoundingClientRect().top <= line) found = s.id; });
    if (found === current) return;
    current = found;
    document.querySelectorAll("header nav a").forEach(function (a) {
      if (a.getAttribute("href") === "#" + current) a.setAttribute("aria-current", "location");
      else a.removeAttribute("aria-current");
    });
  }
  window.addEventListener("scroll", function () { if (!queued) { queued = true; requestAnimationFrame(update); } }, { passive: true });
  update();
})();

// The code window's Copy button copies the code; if the browser refuses, it selects the code instead.
(function () {
  var button = document.querySelector(".code-window .copy"), code = document.querySelector(".code-window code");
  if (!button || !code || !navigator.clipboard) return;
  button.hidden = false;
  var timer;
  function say(text) {
    button.textContent = text;
    clearTimeout(timer);
    timer = setTimeout(function () { button.textContent = "Copy"; }, 1600);
  }
  button.addEventListener("click", function () {
    navigator.clipboard.writeText(code.textContent).then(function () { say("Copied"); }, function () {
      var range = document.createRange();
      range.selectNodeContents(code);
      getSelection().removeAllRanges();
      getSelection().addRange(range);
      say("Press Ctrl+C");
    });
  });
})();
})();
