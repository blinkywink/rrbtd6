// Small touches on the home page. Without this script the page works the same, minus these.
(function () {
var calm = window.matchMedia && window.matchMedia("(prefers-reduced-motion: reduce)").matches;

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

// Progress's bars fill from empty as they scroll up into view (full once 160px in) and stay
// filled until the page is reloaded. The totals add up the tower bars as they fill: matching
// scenarios so far, towers whose bars are full, and the overall bar and percentage from those.
(function () {
  var overall = document.querySelector(".poverall .pbar i"), pct = document.querySelector(".ppct");
  var totals = document.querySelectorAll(".progress-total .pnum");
  if (calm || !overall || !pct || totals.length < 2) return;
  var bars = Array.prototype.map.call(document.querySelectorAll(".prow .pbar i"), function (bar) {
    var row = bar.closest(".prow"), count = /(\d+)\s*\/\s*(\d+)/.exec(row.querySelector(".pcount").textContent);
    var tower = row.parentNode.matches(".poverall + .prows") && count;
    return { bar: bar, fill: 0, tower: !!tower, done: row.classList.contains("done"), matched: tower ? +count[1] : 0, of: tower ? +count[2] : 0 };
  });
  var matched = 0, all = 0;
  bars.forEach(function (b) { matched += b.matched; all += b.of; });
  if (!matched || !all) return;
  var queued = false;
  function update() {
    queued = false;
    var sum = 0, full = 0;
    bars.forEach(function (b) {
      var t = Math.min(1, Math.max(0, (window.innerHeight - b.bar.getBoundingClientRect().top) / 160));
      var f = 1 - (1 - t) * (1 - t);
      if (f > b.fill) { b.fill = f; b.bar.style.scale = f + " 1"; }
      sum += b.matched * b.fill;
      if (b.tower && b.done && b.fill > 0.99) full++;
    });
    totals[0].textContent = Math.round(sum);
    totals[1].textContent = full;
    overall.style.scale = sum / matched + " 1";
    pct.textContent = Math.round(100 * sum / all) + "%";
  }
  function queue() { if (!queued) { queued = true; requestAnimationFrame(update); } }
  window.addEventListener("scroll", queue, { passive: true });
  window.addEventListener("resize", queue);
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
