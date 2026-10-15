// AlgoDeck ⚡ Koło MAP - Background Service Worker
// Automatyczne wykrywanie pobranych zadań PDF i ZIP (Szkopuł, Koło MAP, OIJ, OI)
// - Przechwytuje zakończenie pobierania pliku .pdf lub .zip
// - Zapisuje plik i wysyła do poczekalni (staging) daemona AlgoDeck
// - ZERO automatycznego otwierania wyskakujących okien (koniec problemu z wieloma oknami)
// - Wyświetla powiadomienie systemowe oraz zieloną odznakę "1" na ikonie rozszerzenia

const API_BASE = "http://127.0.0.1:8080";

// Sprzątanie ewentualnych starych zombi-okien przywróconych przez przeglądarkę
async function cleanupRestoredWindows() {
  try {
    if (typeof chrome !== "undefined" && chrome.windows && chrome.windows.getAll) {
      const windows = await chrome.windows.getAll({ populate: true });
      for (const win of windows) {
        for (const tab of win.tabs || []) {
          if (tab.url && tab.url.includes("popup.html")) {
            if (win.tabs.length === 1 || tab.url.includes("auto_pdf=")) {
              console.log("[AlgoDeck] Zamykam przywrócone zombi-okno popup:", win.id);
              chrome.windows.remove(win.id).catch(() => {});
              break;
            }
          }
        }
      }
    }
  } catch (_) {}
}
cleanupRestoredWindows();

// Weryfikacja czy plik to treść zadania PDF
function isOlympicTaskFile(item) {
  if (!item) return false;
  const filename = (item.filename || "").toLowerCase();
  const url = (item.url || "").toLowerCase().split("?")[0];
  const mime = (item.mime || "").toLowerCase();

  if (filename.endsWith(".crdownload") || filename.endsWith(".tmp")) {
    return false;
  }

  // Wyłącznie pliki .pdf! ZIPy nie są automatycznie zaciągane z pobrań
  return filename.endsWith(".pdf") || mime.includes("pdf") || url.endsWith(".pdf");
}

// Sprawdzenie czy usługa AlgoDeck działa
async function isDaemonRunning() {
  try {
    const controller = new AbortController();
    const timer = setTimeout(() => controller.abort(), 1000);
    const res = await fetch(`${API_BASE}/api/status`, {
      signal: controller.signal,
      cache: "no-store"
    });
    clearTimeout(timer);
    return res.ok;
  } catch (_) {
    return false;
  }
}

// Obsługa zakończonego pobierania (wyłącznie PDF)
async function handleCompletedDownload(downloadId) {
  if (!downloadId) return;

  try {
    const items = await chrome.downloads.search({ id: downloadId });
    if (!items || items.length === 0) return;
    const item = items[0];

    if (!isOlympicTaskFile(item)) return;

    // Sprawdź czy automatyczne wykrywanie nie zostało wyłączone w opcjach
    try {
      const stored = await chrome.storage.local.get(["auto_detect_downloads"]);
      if (stored.auto_detect_downloads === false) return;
    } catch (_) {}

    const filename = item.filename || "";
    if (!filename) return;

    const baseName = filename.replace(/^.*[\\\/]/, '');

    console.log("[AlgoDeck] Wykryto pobranie zadania PDF:", filename);

    // 1. Zapisz w chrome.storage.local (tylko ten najnowszy plik)
    await chrome.storage.local.set({
      last_downloaded_pdf: filename,
      last_pdf_name: baseName,
      last_pdf_ts: Date.now()
    });
    // Nowe pobranie unieważnia ewentualne poprzednie wyczyszczenie
    await chrome.storage.local.remove(["cleared_pdf_name"]);

    // 2. Jeśli usługa AlgoDeck działa, wyślij natychmiast do stagingu w tle!
    const isOnline = await isDaemonRunning();
    if (isOnline) {
      try {
        const res = await fetch(`${API_BASE}/api/stage-by-path`, {
          method: "POST",
          headers: { "Content-Type": "application/json" },
          body: JSON.stringify({ file_path: filename })
        });
        if (res.ok) {
          console.log("[AlgoDeck] Plik pomyślnie zaciągnięty do poczekalni (staging) w tle:", filename);
        }
      } catch (err) {
        console.warn("[AlgoDeck] Błąd przesyłania do /api/stage-by-path:", err);
      }
    }

    // 3. Ustaw zieloną odznakę "1" na ikonie paska rozszerzenia
    try {
      if (chrome.action && chrome.action.setBadgeText) {
        chrome.action.setBadgeText({ text: "1" });
        chrome.action.setBadgeBackgroundColor({ color: "#10b981" });
      }
    } catch (_) {}

    // 4. Powiadomienie systemowe
    try {
      if (chrome.notifications && chrome.notifications.create) {
        chrome.notifications.create(`algodeck_${downloadId}_${Date.now()}`, {
          type: "basic",
          iconUrl: "icons/icon128.png",
          title: "AlgoDeck ⚡ Zaciągnięto zadanie",
          message: `Pobrano: ${baseName}\nPlik czeka w poczekalni AlgoDeck. Kliknij ikonę ⚡, aby otworzyć VS Code.`,
          priority: 2,
          requireInteraction: false
        });
      }
    } catch (_) {}
  } catch (e) {
    console.warn("[AlgoDeck] Błąd handleCompletedDownload:", e);
  }
}

// Nasłuchiwanie na żywo zdarzeń pobierania
if (typeof chrome !== "undefined" && chrome.downloads) {
  if (chrome.downloads.onCreated) {
    chrome.downloads.onCreated.addListener((item) => {
      if (item && item.state === "complete") {
        handleCompletedDownload(item.id);
      }
    });
  }

  if (chrome.downloads.onChanged) {
    chrome.downloads.onChanged.addListener((delta) => {
      if (delta && delta.id && delta.state && delta.state.current === "complete") {
        handleCompletedDownload(delta.id);
      }
    });
  }
}

// Kliknięcie w powiadomienie systemowe
if (typeof chrome !== "undefined" && chrome.notifications && chrome.notifications.onClicked) {
  chrome.notifications.onClicked.addListener((notifId) => {
    if (notifId && notifId.startsWith("algodeck_")) {
      chrome.notifications.clear(notifId);
    }
  });
}

// Synchronizacja ustawień
async function syncSettingsFromBackend() {
  try {
    const res = await fetch(`${API_BASE}/api/settings`);
    if (res.ok) {
      const data = await res.json();
      if (data.auto_detect_downloads !== undefined) {
        chrome.storage.local.set({ auto_detect_downloads: data.auto_detect_downloads });
      }
    }
  } catch (_) {}
}
syncSettingsFromBackend();
setInterval(syncSettingsFromBackend, 15000);
