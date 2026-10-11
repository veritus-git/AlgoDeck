// AlgoDeck ⚡ Koło MAP - Background Service Worker
// Niezawodne wykrywanie nowych pobrań PDF i ZIP (Szkopuł, Koło MAP, OIJ, OI)
// Reaguje WYŁĄCZNIE na nowo pobrane pliki w bieżącej sesji!

const API_BASE = "http://127.0.0.1:8080";
const SERVICE_WORKER_START_TIME = Date.now();
const handledDownloads = new Set();

// Wczytaj wcześniej obsłużone identyfikatory z pamięci trwałej
chrome.storage.local.get(["seen_downloads"], (res) => {
  if (Array.isArray(res.seen_downloads)) {
    for (const id of res.seen_downloads) {
      handledDownloads.add(id);
    }
  }
});

function markHandled(id) {
  handledDownloads.add(id);
  chrome.storage.local.get(["seen_downloads"], (res) => {
    const list = Array.isArray(res.seen_downloads) ? res.seen_downloads : [];
    if (!list.includes(id)) {
      list.push(id);
      // Zachowaj maksymalnie 100 ostatnich id
      const trimmed = list.slice(-100);
      chrome.storage.local.set({ seen_downloads: trimmed });
    }
  });
}

// Pomocnicza funkcja sprawdzająca czy plik to zadanie olimpijskie (PDF lub ZIP z testami)
function isOlympicTaskFile(item) {
  if (!item) return false;
  const filename = (item.filename || "").toLowerCase();
  const url = (item.url || "").toLowerCase().split("?")[0];
  const mime = (item.mime || "").toLowerCase();

  const isPdf = filename.endsWith(".pdf") || mime.includes("pdf") || url.endsWith(".pdf");
  const isZip = filename.endsWith(".zip") || mime.includes("zip") || mime.includes("compressed") || url.endsWith(".zip");

  // Wykluczamy pliki tymczasowe przeglądarki
  if (filename.endsWith(".crdownload") || filename.endsWith(".tmp")) {
    return false;
  }

  return isPdf || isZip;
}

// Główna funkcja przetwarzania zakończonego pobierania
async function processDownload(downloadId) {
  if (handledDownloads.has(downloadId)) return;

  chrome.downloads.search({ id: downloadId }, async (items) => {
    if (!items || items.length === 0) return;
    const item = items[0];

    // Ignorujemy pliki pobrane przed startem tej sesji rozszerzenia (np. po resecie komputera)
    const startTime = item.startTime ? new Date(item.startTime).getTime() : 0;
    if (startTime > 0 && startTime < (SERVICE_WORKER_START_TIME - 2000)) {
      markHandled(downloadId);
      return;
    }

    // Jeśli jeszcze trwa pobieranie, ponów za chwilę (retry z backoffem)
    if (item.state !== "complete") {
      setTimeout(() => {
        if (!handledDownloads.has(downloadId)) {
          processDownload(downloadId);
        }
      }, 500);
      return;
    }

    let filename = item.filename || "";
    if (filename.endsWith(".crdownload") || filename.endsWith(".tmp")) {
      setTimeout(() => processDownload(downloadId), 400);
      return;
    }

    if (!isOlympicTaskFile(item)) {
      return;
    }

    // Sprawdź czy automatyczne wykrywanie nie zostało wyłączone w ustawieniach
    let autoDetect = true;
    try {
      const stored = await chrome.storage.local.get(["auto_detect_downloads"]);
      if (stored.auto_detect_downloads === false) {
        autoDetect = false;
      }
    } catch (e) {}

    if (!autoDetect) {
      console.log("[AlgoDeck] Automatyczne wykrywanie pobrań wyłączone w ustawieniach.");
      return;
    }

    if (handledDownloads.has(downloadId)) return;
    markHandled(downloadId);

    console.log("[AlgoDeck] Wykryto nowo pobrany plik zadania:", filename);

    // 1. Zapisz w pamięci podręcznej rozszerzenia
    try {
      await chrome.storage.local.set({
        auto_pdf: filename,
        auto_pdf_ts: Date.now(),
        auto_is_zip: filename.toLowerCase().endsWith(".zip")
      });
    } catch (e) {}

    // 2. Wyślij do stagingu na backendzie (jeśli serwer działa)
    try {
      await fetch(`${API_BASE}/api/auto-import-pdf`, {
        method: "POST",
        headers: { "Content-Type": "application/json" },
        body: JSON.stringify({ file_path: filename, auto_create: false })
      });
    } catch (err) {
      console.warn("[AlgoDeck] Serwer nie odpowiada jeszcze na staging:", err);
    }

    // 3. Pokaż powiadomienie systemowe Chrome (Native OS Toast)
    const baseName = filename.replace(/^.*[\\\/]/, '');
    try {
      if (chrome.notifications && chrome.notifications.create) {
        chrome.notifications.create(`algodeck_${downloadId}`, {
          type: "basic",
          iconUrl: "icons/icon128.png",
          title: "AlgoDeck ⚡ Wykryto zadanie olimpijskie",
          message: `Pobrano: ${baseName}\nKliknij, aby błyskawicznie utworzyć workspace C++20!`,
          priority: 2,
          requireInteraction: true
        });
      }
    } catch (e) {}

    // 4. Otwórz dedykowane okienko powiadomienia
    openDedicatedNotificationWindow(filename);
  });
}

// Otwieranie dedykowanego okienka powiadomienia 1-kliknięcie
function openDedicatedNotificationWindow(filename) {
  const targetUrl = chrome.runtime.getURL(`popup.html?auto_pdf=${encodeURIComponent(filename)}`);
  
  chrome.windows.create({
    url: targetUrl,
    type: "popup",
    width: 380,
    height: 165,
    focused: true
  }).catch((err) => {
    console.warn("[AlgoDeck] Błąd otwierania okna popupa:", err);
  });
}

// Synchronizacja ustawień z backendem
async function syncSettingsFromBackend() {
  try {
    const res = await fetch(`${API_BASE}/api/settings`);
    if (res.ok) {
      const data = await res.json();
      if (data.auto_detect_downloads !== undefined) {
        chrome.storage.local.set({ auto_detect_downloads: data.auto_detect_downloads });
      }
    }
  } catch (e) {}
}
syncSettingsFromBackend();
setInterval(syncSettingsFromBackend, 15000);

// Nasłuchiwanie zmian w pobieraniach
chrome.downloads.onChanged.addListener((delta) => {
  if (delta.state && delta.state.current === "complete") {
    processDownload(delta.id);
  } else if (delta.filename && !delta.filename.current.endsWith(".crdownload")) {
    processDownload(delta.id);
  }
});

chrome.downloads.onCreated.addListener((item) => {
  if (item.state === "complete") {
    processDownload(item.id);
  }
});

// Kliknięcie w powiadomienie systemowe
if (chrome.notifications && chrome.notifications.onClicked) {
  chrome.notifications.onClicked.addListener((notifId) => {
    chrome.storage.local.get(["auto_pdf"], (data) => {
      if (data.auto_pdf) {
        openDedicatedNotificationWindow(data.auto_pdf);
      }
    });
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
        console.log("[AlgoDeck] Nowy build_id, przeładowuję rozszerzenie...");
        chrome.runtime.reload();
      }
    }
  } catch (e) {}
}, 2500);
