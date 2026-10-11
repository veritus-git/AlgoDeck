// AlgoDeck ⚡ Koło MAP - Background Service Worker
// Automatyczne profesjonalne wykrywanie pobranych plików PDF i natychmiastowe otwieranie okienka rozszerzenia (1-Klik)

const API_BASE = "http://127.0.0.1:8080";
const handledDownloads = new Set();

async function processDownload(downloadId) {
  if (handledDownloads.has(downloadId)) return;

  chrome.downloads.search({ id: downloadId }, async (items) => {
    if (!items || items.length === 0) return;
    const item = items[0];

    // Jeśli jeszcze trwa pobieranie, poczekaj na zdarzenie complete
    if (item.state !== 'complete') {
      return;
    }

    let filename = item.filename || "";
    if (filename.endsWith('.crdownload')) {
      // Jeśli plik ma jeszcze tymczasowe rozszerzenie przeglądarki, ponów za 250ms
      setTimeout(() => processDownload(downloadId), 250);
      return;
    }

    const isPdf = filename.toLowerCase().endsWith('.pdf') ||
                  (item.mime && item.mime.toLowerCase().includes('pdf')) ||
                  (item.url && item.url.toLowerCase().split('?')[0].endsWith('.pdf'));

    if (!isPdf) return;

    if (handledDownloads.has(downloadId)) return;
    handledDownloads.add(downloadId);

    console.log("[AlgoDeck] Wykryto pobrany plik PDF:", filename);

    // 1. Zapisz w stagingu na backendzie
    try {
      await fetch(`${API_BASE}/api/auto-import-pdf`, {
        method: "POST",
        headers: { "Content-Type": "application/json" },
        body: JSON.stringify({ file_path: filename, auto_create: false })
      });
    } catch (err) {
      console.warn("[AlgoDeck] Błąd wstępnego stagingu:", err);
    }

    // 2. Zapisz ścieżkę do pamięci podręcznej rozszerzenia
    try {
      await chrome.storage.local.set({
        auto_pdf: filename,
        auto_pdf_ts: Date.now()
      });
    } catch (e) {}

    // 3. Natychmiast otwórz okienko rozszerzenia
    openExtensionPopup(filename);
  });
}

chrome.downloads.onChanged.addListener((delta) => {
  if (delta.state && delta.state.current === 'complete') {
    processDownload(delta.id);
  } else if (delta.filename && !delta.filename.current.endsWith('.crdownload')) {
    processDownload(delta.id);
  }
});

chrome.downloads.onCreated.addListener((item) => {
  if (item.state === 'complete') {
    processDownload(item.id);
  }
});

async function openExtensionPopup(filename) {
  let opened = false;

  // 1. Spróbuj otworzyć natywne okienko popupa z paska narzędzi przeglądarki
  if (chrome.action && chrome.action.openPopup) {
    try {
      const windows = await chrome.windows.getAll({ windowTypes: ['normal'] });
      const focusedWin = windows.find(w => w.focused) || windows[0];
      if (focusedWin && focusedWin.id) {
        await chrome.action.openPopup({ windowId: focusedWin.id });
      } else {
        await chrome.action.openPopup();
      }
      opened = true;
    } catch (e) {
      console.log("[AlgoDeck] chrome.action.openPopup wymagał gestu, otwieram dedykowane okno:", e);
      opened = false;
    }
  }

  // 2. Jeśli polityka Chromium wymaga gestu użytkownika na pasku, otwórz dedykowane okno rozszerzenia
  if (!opened) {
    openDedicatedWindow(filename);
  }
}

function openDedicatedWindow(filename) {
  const targetUrl = chrome.runtime.getURL(`popup.html?auto_pdf=${encodeURIComponent(filename)}`);
  chrome.windows.create({
    url: targetUrl,
    type: "popup",
    width: 440,
    height: 580,
    focused: true
  }).catch(err => {
    console.warn("[AlgoDeck] Błąd otwierania okna popupa:", err);
  });
}

// Auto-reload rozszerzenia w tle podczas aktualizacji kodu
let currentBuildId = null;
setInterval(async () => {
  try {
    const res = await fetch(`${API_BASE}/api/extension-reload-id`);
    if (res.ok) {
      const data = await res.json();
      if (currentBuildId === null) {
        currentBuildId = data.build_id;
      } else if (currentBuildId !== data.build_id) {
        console.log("[AlgoDeck] Wykryto nową wersję, przeładowuję rozszerzenie...");
        chrome.runtime.reload();
      }
    }
  } catch (e) {}
}, 2500);
