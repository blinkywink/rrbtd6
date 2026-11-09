// On a Mac or Linux the main button leads to the build instructions instead (the installer is for
// Windows), and the line under it offers the Windows download.
(function () {
  var hero = document.querySelector(".hero a.download"), also = document.querySelector(".hero .fine:not(.release)");
  if (!hero || !also) return;
  var platform = ((navigator.userAgentData && navigator.userAgentData.platform) || navigator.platform || "").toLowerCase();
  var agent = navigator.userAgent.toLowerCase();
  if (/android|iphone|ipad|ipod/.test(agent) || (/mac/.test(platform) && navigator.maxTouchPoints > 1)) return;
  var os = /mac/.test(platform) ? "macOS" : /linux/.test(platform) ? "Linux" : null;
  if (!os) return;
  var installer = hero.href;
  hero.firstChild.textContent = "Build for " + os + " ";
  hero.href = "#platforms";
  hero.classList.add("for-build");
  also.innerHTML = 'On Windows? <a href="' + installer + '">Download the installer</a>';
})();

// Download, then show how to install: the button still starts the download (its href), and a
// moment later the page scrolls to the install steps.
(function () {
  var install = document.getElementById("install");
  if (!install) return;
  var calm = window.matchMedia && window.matchMedia("(prefers-reduced-motion: reduce)").matches;
  document.querySelectorAll('a.download:not(.for-build), .hero .fine a[href$=".dll"]').forEach(function (a) {
    a.addEventListener("click", function () {
      setTimeout(function () {
        install.scrollIntoView({ behavior: calm ? "auto" : "smooth", block: "start" });
      }, 400);
    });
  });
})();

// Under the button: the latest build's number, age and installer size, from GitHub's API (kept for
// ten minutes per tab). Every push to main publishes a new build, so this can't be written into the
// page; when the request fails the line just stays hidden.
(function () {
  var line = document.getElementById("release");
  if (!line || !window.fetch) return;
  var key = "rrbtd6-release", cached = null;
  try { cached = JSON.parse(sessionStorage.getItem(key)); } catch (e) {}
  if (cached && Date.now() - cached.at < 600000) return show(cached.release);
  fetch("https://api.github.com/repos/blinkywink/rrbtd6/releases/latest", { headers: { Accept: "application/vnd.github+json" } })
    .then(function (r) { if (!r.ok) throw r.status; return r.json(); })
    .then(function (data) {
      var asset = (data.assets || []).filter(function (a) { return a.name === "RRBTD6Installer.dll"; })[0];
      var release = { name: data.name, published: data.published_at, size: asset ? asset.size : 0 };
      try { sessionStorage.setItem(key, JSON.stringify({ at: Date.now(), release: release })); } catch (e) {}
      show(release);
    })
    .catch(function () {});

  function show(release) {
    var parts = [release.name, "updated " + ago(new Date(release.published))];
    if (release.size) parts.push(Math.max(1, Math.round(release.size / 1024)) + " KB installer");
    line.textContent = parts.join(" · ");
    line.hidden = false;
  }
  function ago(date) {
    var seconds = (date - Date.now()) / 1000, units = [["day", 86400], ["hour", 3600], ["minute", 60]];
    var format = window.Intl && Intl.RelativeTimeFormat ? new Intl.RelativeTimeFormat("en", { numeric: "auto" }) : null;
    for (var i = 0; i < units.length; i++) {
      if (Math.abs(seconds) >= units[i][1] || i === units.length - 1) {
        var n = Math.round(seconds / units[i][1]);
        return format ? format.format(n, units[i][0]) : Math.abs(n) + " " + units[i][0] + (Math.abs(n) === 1 ? "" : "s") + " ago";
      }
    }
  }
})();
