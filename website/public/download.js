// Next to the install guide's RustTD6 button: the current version ("Version 0.651", the release title), its age and the
// installer size, from GitHub's API (kept for ten minutes per tab). A release is published whenever
// VERSION is raised, so this isn't written into the page; when the request fails the line stays
// hidden.
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
