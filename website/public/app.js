// Optional release metadata; never replace the stable /releases/latest/download link.
async function releaseInfo() {
  const label = document.querySelector('[data-release]');
  try {
    const response = await fetch('https://api.github.com/repos/blinkywink/rrbtd6/releases/latest', {
      headers: { Accept: 'application/vnd.github+json' }, signal: AbortSignal.timeout(5000),
    });
    if (!response.ok) return;
    const release = await response.json();
    if (release.draft || release.prerelease || !release.tag_name) return;
    const installer = release.assets?.find(asset => asset.name === 'RRBTD6Installer.dll');
    const bytes = Number(installer?.size);
    const size = Number.isFinite(bytes) && bytes > 0 ? ` · ${Math.ceil(bytes / 1024)} KB installer` : '';
    label.textContent = `${release.name || release.tag_name}${size}`;
    const client = release.assets?.find(asset => asset.name === 'rrbtd6.exe');
    const clientBytes = Number(client?.size);
    if (Number.isFinite(clientBytes) && clientBytes > 0) {
      document.querySelector('[data-client-size]').textContent = `~${Math.ceil(clientBytes / 1e6)} MB client · Python/tools downloaded separately`;
    }
  } catch { /* Offline, rate limited, or older browser: keep the working fallback. */ }
}
releaseInfo();
