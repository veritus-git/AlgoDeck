// AlgoDeck ⚡ Koło MAP - Background Service Worker
// Niezawodne, rygorystyczne wykrywanie ŚWIEŻO pobranych zadań PDF i ZIP (Szkopuł, Koło MAP, OIJ, OI)
// - Wykorzystuje znacznik czasu startu sesji (SW_BOOT_TIME) do bezwzględnego odrzucania starych pobrań
// - Odtwarzanie historii przeglądarki po restarcie jest w 100% filtrowane
// - Singleton lock na otwieranie dedykowanego okna (nigdy więcej niż 1 okienko na ekranie)

const API_BASE = "http://127.0.0.1:8080";

// Czas startu bieżącego Service Workera
const SW_BOOT_TIME = Date.now();

let isSessionReady = false;
let initPromise = null;

// Pobrania zainicjowane NA ŻYWO w tej sesji (onCreated w stanie in_progress po SW_BOOT_TIME)
const sessionCreatedDownloads = new Set();
const handledDownloads = new Set();
let seenFilesMap = {};

let activeNotificationWindowId = null;
let notificationWindowPromise = null;

// 1. Inicjalizacja: zindeksuj historię pobrań, by żadne stare pobranie nie wywołało powiadomienia
async function ensureSessionInitialized() {
  if (isSessionReady) return;
  if (initPromise) return initPromise;

  initPromise = (async () => {
    try {
      // Wyczyść ewentualne stare stany tymczasowe
      try {
        if (typeof chrome !== "undefined" && chrome.storage && chrome.storage.local) {
          const stored = await chrome.storage.local.get(["seen_files", "seen_downloads"]);
          if (stored.seen_files && typeof stored.seen_files === "object") {
            seenFilesMap = stored.seen_files;
          }
          if (Array.isArray(stored.seen_downloads)) {
            for (const id of stored.seen_downloads) {
              handledDownloads.add(id);
            }
          }
        }
      } catch (_) {}

      // Zindeksuj WSZYSTKIE istniejące pobrania w historii przeglądarki jako obsłużone
      if (typeof chrome !== "undefined" && chrome.downloads && chrome.downloads.search) {
        try {
          const existing = await chrome.downloads.search({});
          for (const item of existing) {
            if (item.id) handledDownloads.add(item.id);
            if (item.filename) {
              const normKey = item.filename.toLowerCase();
              seenFilesMap[normKey] = {
                downloadId: item.id,
                time: item.startTime ? new Date(item.startTime).getTime() : 0,
                handled: true
              };
            }
          }
        } catch (_) {}
      }

      // Zapisz zaktualizowaną listę obsłużonych
      try {
        if (typeof chrome !== "undefined" && chrome.storage && chrome.storage.local) {
          const entries = Object.entries(seenFilesMap).slice(-300);
          seenFilesMap = Object.fromEntries(entries);
          await chrome.storage.local.set({
            seen_downloads: Array.from(handledDownloads).slice(-300),
            seen_files: seenFilesMap
          });
        }
      } catch (_) {}
    } catch (err) {
      console.warn("[AlgoDeck] Błąd inicjalizacji sesji:", err);
    } finally {
      isSessionReady = true;
    }
  })();

  return initPromise;
}

ensureSessionInitialized();

async function markHandled(downloadId, filename, url) {
  handledDownloads.add(downloadId);
  sessionCreatedDownloads.delete(downloadId);

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
    if (typeof chrome !== "undefined" && chrome.storage && chrome.storage.local) {
      const dList = Array.from(handledDownloads).slice(-300);
      const entries = Object.entries(seenFilesMap).slice(-300);
      seenFilesMap = Object.fromEntries(entries);
      await chrome.storage.local.set({
        seen_downloads: dList,
        seen_files: seenFilesMap
      });
    }
  } catch (_) {}
}

function isOlympicTaskFile(item) {
  if (!item) return false;
  const filename = (item.filename || "").toLowerCase();
  const url = (item.url || "").toLowerCase().split("?")[0];
  const mime = (item.mime || "").toLowerCase();

  const isPdf = filename.endsWith(".pdf") || mime.includes("pdf") || url.endsWith(".pdf");
  const isZip = filename.endsWith(".zip") || mime.includes("zip") || mime.includes("compressed") || url.endsWith(".zip");

  if (filename.endsWith(".crdownload") || filename.endsWith(".tmp")) {
    return false;
  }

  return isPdf || isZip;
}

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
  } catch (_) {}
  return false;
}

// Główna funkcja przetwarzania nowo zakończonego pobrania
async function processDownload(downloadId) {
  if (handledDownloads.has(downloadId)) return;

  chrome.downloads.search({ id: downloadId }, async (items) => {
    if (!items || items.length === 0) return;
    const item = items[0];

    const filename = item.filename || "";
    const normKey = filename.toLowerCase();

    // 1. Sprawdź czy plik był już widziany
    if (seenFilesMap[normKey]) {
      markHandled(downloadId, filename, item.url);
      return;
    }

    // 2. Walidacja czasu: pobranie MUSIAŁO rozpocząć się po starcie tego Service Workera
    const startTime = item.startTime ? new Date(item.startTime).getTime() : 0;
    if (startTime <= SW_BOOT_TIME + 500) {
      markHandled(downloadId, filename, item.url);
      return;
    }

    // 3. Sprawdź czy pobieranie jest kompletne
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
      markHandled(downloadId, filename, item.url);
      return;
    }

    // 4. Sprawdź czy automatyczne wykrywanie nie zostało wyłączone
    try {
      const stored = await chrome.storage.local.get(["auto_detect_downloads"]);
      if (stored.auto_detect_downloads === false) {
        markHandled(downloadId, filename, item.url);
        return;
      }
    } catch (_) {}

    // 5. Sprawdź czy zadanie już istnieje w workspace
    const alreadyExists = await isProblemAlreadyInWorkspace(filename);
    if (alreadyExists) {
      console.log("[AlgoDeck] Zadanie z pliku", filename, "już istnieje w workspace. Pomijam powiadomienie.");
      markHandled(downloadId, filename, item.url);
      return;
    }

    if (handledDownloads.has(downloadId) || seenFilesMap[normKey]) return;
    await markHandled(downloadId, filename, item.url);

    console.log("[AlgoDeck] Pomyślnie wykryto NOWE pobranie zadania:", filename);

    // Zapisz w storage rozszerzenia
    try {
      await chrome.storage.local.set({
        auto_pdf: filename,
        auto_pdf_ts: Date.now(),
        auto_is_zip: filename.toLowerCase().endsWith(".zip")
      });
    } catch (_) {}

    // Oznacz odznakę na ikonie rozszerzenia
    try {
      if (chrome.action && chrome.action.setBadgeText) {
        chrome.action.setBadgeText({ text: "1" });
        chrome.action.setBadgeBackgroundColor({ color: "#10b981" });
      }
    } catch (_) {}

    // Powiadomienie natywne Chrome
    const baseName = filename.replace(/^.*[\\\/]/, '');
    try {
      if (chrome.notifications && chrome.notifications.create) {
        chrome.notifications.create(`algodeck_${downloadId}`, {
          type: "basic",
          iconUrl: "icons/icon128.png",
          title: "AlgoDeck ⚡ Wykryto zadanie olimpijskie",
          message: `Pobrano: ${baseName}\nKliknij, aby utworzyć zadanie w AlgoDeck.`,
          priority: 2,
          requireInteraction: false
        });
      }
    } catch (_) {}

    // Otwórz DOKŁADNIE JEDNO okienko 1-kliknięcia
    await openDedicatedNotificationWindow(filename);
  });
}

// ------------------------------------------------------------------------------
// PANCERNY SINGLETON OKNA POWIADOMIENIA (Zero duplikatów)
// ------------------------------------------------------------------------------
async function openDedicatedNotificationWindow(filename) {
  if (!filename) return;

  const targetUrl = chrome.runtime.getURL(`popup.html?auto_pdf=${encodeURIComponent(filename)}`);

  // 1. Sprawdź czy JAKIEKOLWIEK okno z popup.html jest już otwarte w przeglądarce
  try {
    const windows = await chrome.windows.getAll({ populate: true });
    for (const win of windows) {
      if (win.type === "popup") {
        for (const tab of win.tabs || []) {
          if (tab.url && tab.url.includes("popup.html")) {
            // Zamiast tworzyć nowe okno, zaktualizuj istniejące i wysuń na wierzch!
            await chrome.tabs.update(tab.id, { url: targetUrl });
            await chrome.windows.update(win.id, { focused: true });
            activeNotificationWindowId = win.id;
            return;
          }
        }
      }
    }
  } catch (_) {}

  // 2. Jeśli inne wywołanie właśnie tworzy okno, poczekaj na jego ukończenie
  if (notificationWindowPromise) {
    try {
      await notificationWindowPromise;
    } catch (_) {}
  }

  // 3. Po odczekaniu sprawdź ponownie referencję
  if (activeNotificationWindowId !== null) {
    try {
      const win = await chrome.windows.get(activeNotificationWindowId, { populate: true });
      if (win && win.tabs && win.tabs[0]) {
        await chrome.tabs.update(win.tabs[0].id, { url: targetUrl });
        await chrome.windows.update(activeNotificationWindowId, { focused: true });
        return;
      }
    } catch (_) {
      activeNotificationWindowId = null;
    }
  }

  // 4. Utwórz dokładnie jedną instancję okienka
  notificationWindowPromise = (async () => {
    try {
      const win = await chrome.windows.create({
        url: targetUrl,
        type: "popup",
        width: 405,
        height: 215,
        focused: true
      });
      if (win && win.id) {
        activeNotificationWindowId = win.id;
      }
    } catch (e) {
      console.warn("[AlgoDeck] Błąd tworzenia okna powiadomienia:", e);
    } finally {
      notificationWindowPromise = null;
    }
  })();

  await notificationWindowPromise;
}

// Czyszczenie referencji okna po jego zamknięciu
if (typeof chrome !== "undefined" && chrome.windows && chrome.windows.onRemoved) {
  chrome.windows.onRemoved.addListener((windowId) => {
    if (windowId === activeNotificationWindowId) {
      activeNotificationWindowId = null;
    }
  });
}

// Obsługa kliknięcia w powiadomienie Chrome
if (typeof chrome !== "undefined" && chrome.notifications && chrome.notifications.onClicked) {
  chrome.notifications.onClicked.addListener((notifId) => {
    if (notifId && notifId.startsWith("algodeck_")) {
      chrome.notifications.clear(notifId);
      try {
        if (chrome.action && chrome.action.setBadgeText) {
          chrome.action.setBadgeText({ text: "" });
        }
      } catch (_) {}
      chrome.storage.local.get(["auto_pdf"], (data) => {
        if (data && data.auto_pdf) {
          openDedicatedNotificationWindow(data.auto_pdf);
        } else {
          chrome.windows.create({
            url: chrome.runtime.getURL("popup.html"),
            type: "popup",
            width: 390,
            height: 600,
            focused: true
          });
        }
      });
    }
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
  } catch (_) {}
}
syncSettingsFromBackend();
setInterval(syncSettingsFromBackend, 15000);

// ------------------------------------------------------------------------------
// NASŁUCHIWANIE POBRAŃ — BEZWZGLĘDNE ODRZUCANIE HISTORII
// ------------------------------------------------------------------------------

// onCreated: rejestrujemy WYŁĄCZNIE pobrania rozpoczęte na żywo jako in_progress po SW_BOOT_TIME
if (typeof chrome !== "undefined" && chrome.downloads && chrome.downloads.onCreated) {
  chrome.downloads.onCreated.addListener((item) => {
    if (!item || !item.id) return;

    // Każde pobranie, które w momencie onCreated jest już "complete" lub "interrupted",
    // jest na 100% odtwarzane z historii przeglądarki przy starcie!
    if (item.state !== "in_progress") {
      handledDownloads.add(item.id);
      return;
    }

    // Sprawdź czas rozpoczęcia
    const itemStart = item.startTime ? new Date(item.startTime).getTime() : 0;
    if (itemStart <= SW_BOOT_TIME + 500) {
      handledDownloads.add(item.id);
      return;
    }

    // Pobranie wystartowało rzeczywiście na żywo w tej sesji:
    sessionCreatedDownloads.add(item.id);
  });
}

// onChanged: reagujemy TYLKO na zakończenie pobrań zainicjowanych w tej sesji
if (typeof chrome !== "undefined" && chrome.downloads && chrome.downloads.onChanged) {
  chrome.downloads.onChanged.addListener((delta) => {
    if (!delta || !delta.id) return;

    // Pobranie MUSI być obecne w sessionCreatedDownloads (czyli rozpoczęte na żywo)
    if (!sessionCreatedDownloads.has(delta.id)) {
      handledDownloads.add(delta.id);
      return;
    }

    if (delta.state && delta.state.current === "complete") {
      sessionCreatedDownloads.delete(delta.id);
      processDownload(delta.id);
    }
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
  } catch (_) {}
}, 2500);
