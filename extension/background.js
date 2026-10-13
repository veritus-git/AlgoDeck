// AlgoDeck ⚡ Koło MAP - Background Service Worker
// Niezawodne wykrywanie NOWO pobranych zadań PDF i ZIP (Szkopuł, Koło MAP, OIJ, OI)
// - Ścisłe śledzenie wcześniej pobranych plików (nie dubluje powiadomień po restarcie)
// - Zabezpieczenie przed otwieraniem wielu okienek popupa (dokładnie 1 instancja powiadomienia)

const API_BASE = "http://127.0.0.1:8080";
const SESSION_START_TIME = Date.now();
const handledDownloads = new Set();
let seenFilesMap = {};
let activeNotificationWindowId = null;

// 1. Inicjalizacja: Wczytaj historię z chrome.storage i zindeksuj istniejące pliki
async function initSession() {
  try {
    const res = await chrome.storage.local.get(["seen_files", "seen_downloads"]);
    if (res.seen_files && typeof res.seen_files === "object") {
      seenFilesMap = res.seen_files;
    }
    if (Array.isArray(res.seen_downloads)) {
      for (const id of res.seen_downloads) {
        handledDownloads.add(id);
      }
    }

    // Zindeksuj WSZYSTKIE istniejące już pobrania w historii przeglądarki,
    // aby po restarcie komputera lub ponownym uruchomieniu przeglądarki
    // pliki sprzed restartu NIE były traktowane jako nowe!
    const existing = await chrome.downloads.search({});
    let updated = false;
    for (const item of existing) {
      if (item.id) handledDownloads.add(item.id);
      if (item.filename) {
        const normKey = item.filename.toLowerCase();
        if (!seenFilesMap[normKey]) {
          seenFilesMap[normKey] = {
            startTime: item.startTime || 0,
            endTime: item.endTime || 0,
            url: item.url || "",
            initial: true
          };
          updated = true;
        }
      }
    }
    if (updated) {
      // Zachowaj maksymalnie 300 ostatnich wpisów
      const entries = Object.entries(seenFilesMap).slice(-300);
      seenFilesMap = Object.fromEntries(entries);
      await chrome.storage.local.set({ seen_files: seenFilesMap });
    }
  } catch (err) {
    console.warn("[AlgoDeck] Błąd inicjalizacji sesji pobrań:", err);
  }
}
initSession();

async function markHandled(downloadId, filename, url) {
  handledDownloads.add(downloadId);
  if (filename) {
    const normKey = filename.toLowerCase();
    seenFilesMap[normKey] = {
      downloadId: downloadId,
      time: Date.now(),
      url: url || "",
      handled: true
    };
  }

  try {
    const dList = Array.from(handledDownloads).slice(-200);
    const entries = Object.entries(seenFilesMap).slice(-300);
    seenFilesMap = Object.fromEntries(entries);
    await chrome.storage.local.set({
      seen_downloads: dList,
      seen_files: seenFilesMap
    });
  } catch (e) {}
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

// Sprawdź czy zadanie o danym kodzie nie istnieje już w workspace AlgoDeck
async function isProblemAlreadyInWorkspace(filename) {
  try {
    const base = filename.replace(/^.*[\\\/]/, '').replace(/\.(pdf|zip)$/i, '').toLowerCase();
    const res = await fetch(`${API_BASE}/api/problems`, { cache: "no-store" });
    if (res.ok) {
      const data = await res.json();
      const list = Array.isArray(data.problems) ? data.problems : (Array.isArray(data) ? data : []);
      for (const p of list) {
        const pid = (p.problem_id || "").toLowerCase();
        if (pid && (pid === base || base.startsWith(pid))) {
          return true;
        }
      }
    }
  } catch (e) {}
  return false;
}

// Główna funkcja przetwarzania zakończonego pobierania
async function processDownload(downloadId) {
  if (handledDownloads.has(downloadId)) return;

  chrome.downloads.search({ id: downloadId }, async (items) => {
    if (!items || items.length === 0) return;
    const item = items[0];

    let filename = item.filename || "";
    const normKey = filename.toLowerCase();

    // 1. Sprawdź czy plik był już wcześniej widziany
    if (seenFilesMap[normKey]) {
      markHandled(downloadId, filename, item.url);
      return;
    }

    // 2. Ignorujemy pliki pobrane przed startem tej sesji przeglądarki
    const startTime = item.startTime ? new Date(item.startTime).getTime() : 0;
    if (startTime > 0 && startTime < (SESSION_START_TIME - 3000)) {
      markHandled(downloadId, filename, item.url);
      return;
    }

    // 3. Jeśli jeszcze trwa pobieranie, ponów za chwilę
    if (item.state !== "complete") {
      setTimeout(() => {
        if (!handledDownloads.has(downloadId)) {
          processDownload(downloadId);
        }
      }, 500);
      return;
    }

    if (filename.endsWith(".crdownload") || filename.endsWith(".tmp")) {
      setTimeout(() => processDownload(downloadId), 400);
      return;
    }

    if (!isOlympicTaskFile(item)) {
      return;
    }

    // 4. Sprawdź czy automatyczne wykrywanie nie zostało wyłączone
    let autoDetect = true;
    try {
      const stored = await chrome.storage.local.get(["auto_detect_downloads"]);
      if (stored.auto_detect_downloads === false) {
        autoDetect = false;
      }
    } catch (e) {}

    if (!autoDetect) {
      console.log("[AlgoDeck] Automatyczne wykrywanie pobrań wyłączone.");
      return;
    }

    // 5. Sprawdź czy zadanie już nie istnieje w workspace
    const alreadyExists = await isProblemAlreadyInWorkspace(filename);
    if (alreadyExists) {
      console.log("[AlgoDeck] Zadanie z pliku", filename, "już istnieje w workspace. Pomijam modal.");
      markHandled(downloadId, filename, item.url);
      return;
    }

    if (handledDownloads.has(downloadId) || seenFilesMap[normKey]) return;
    await markHandled(downloadId, filename, item.url);

    console.log("[AlgoDeck] Wykryto nowo pobrany plik zadania:", filename);

    // Zapisz w pamięci podręcznej rozszerzenia
    try {
      await chrome.storage.local.set({
        auto_pdf: filename,
        auto_pdf_ts: Date.now(),
        auto_is_zip: filename.toLowerCase().endsWith(".zip")
      });
    } catch (e) {}

    // Wyślij do stagingu na backendzie
    try {
      await fetch(`${API_BASE}/api/auto-import-pdf`, {
        method: "POST",
        headers: { "Content-Type": "application/json" },
        body: JSON.stringify({ file_path: filename, auto_create: false })
      });
    } catch (err) {
      console.warn("[AlgoDeck] Serwer nie odpowiada jeszcze na staging:", err);
    }

    // Powiadomienie systemowe Chrome (Native OS Toast)
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

    // Otwórz DOKŁADNIE JEDNO dedykowane okienko powiadomienia (singleton)
    await openDedicatedNotificationWindow(filename);
  });
}

// Otwieranie dedykowanego okienka powiadomienia 1-kliknięcie (Ściśle JEDNA instancja okna)
async function openDedicatedNotificationWindow(filename) {
  const targetUrl = chrome.runtime.getURL(`popup.html?auto_pdf=${encodeURIComponent(filename)}`);

  if (activeNotificationWindowId !== null) {
    try {
      const existingWin = await chrome.windows.get(activeNotificationWindowId);
      if (existingWin) {
        const tabs = await chrome.tabs.query({ windowId: activeNotificationWindowId });
        if (tabs && tabs.length > 0) {
          await chrome.tabs.update(tabs[0].id, { url: targetUrl });
          await chrome.windows.update(activeNotificationWindowId, { focused: true });
          return;
        }
      }
    } catch (_) {
      activeNotificationWindowId = null;
    }
  }

  try {
    const newWin = await chrome.windows.create({
      url: targetUrl,
      type: "popup",
      width: 380,
      height: 165,
      focused: true
    });
    activeNotificationWindowId = newWin.id;
  } catch (err) {
    console.warn("[AlgoDeck] Błąd otwierania okna popupa:", err);
  }
}

// Czyszczenie referencji okna po jego zamknięciu
chrome.windows.onRemoved.addListener((windowId) => {
  if (windowId === activeNotificationWindowId) {
    activeNotificationWindowId = null;
  }
});

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

// Nasłuchiwanie zdarzeń pobierania
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
