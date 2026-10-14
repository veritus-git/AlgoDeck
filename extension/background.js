// AlgoDeck ⚡ Koło MAP - Background Service Worker
// Niezawodne, rygorystyczne wykrywanie ŚWIEŻO pobranych zadań PDF i ZIP (Szkopuł, Koło MAP, OIJ, OI)
// - Wykorzystuje twardy okres wyciszenia startowego (STARTUP_MUTE_MS = 10s) na odrzucenie jakiejkolwiek historii
// - Zamyka osierocone / przywrócone okna popup z poprzedniej sesji przeglądarki (Session Restore)
// - Blokada singletonowa i cooldown na tworzenie okien (max 1 okno powiadomienia, zero spamu)

const API_BASE = "http://127.0.0.1:8080";

// Czas startu Service Workera
const SW_BOOT_TIME = Date.now();

// 1. BEZWZGLĘDNY OKRES WYCISZENIA STARTOWEGO (10 sekund)
// W ciągu pierwszych 10 sekund po uruchomieniu rozszerzenia/przeglądarki
// żadne powiadomienie ani okno nie ma prawa powstać.
const STARTUP_MUTE_MS = 10000;

// 2. COOLDOWN TWORZENIA OKIEN (10 sekund)
const WINDOW_COOLDOWN_MS = 10000;
let lastWindowOpenedTime = 0;
let isOpeningWindow = false;
let activeNotificationWindowId = null;

// Rejestry pobrań
// liveDownloads: ID -> timestamp zarejestrowany na żywo w bieżącej sesji po upływie mute
const liveDownloads = new Map();
const handledDownloads = new Set();
let seenFilesMap = {};

// ------------------------------------------------------------------------------
// SPRZĄTANIE PRZYWRÓCONYCH OKIEN SESJI (Brave/Chrome "Continue where you left off")
// ------------------------------------------------------------------------------
async function cleanupRestoredWindows() {
  try {
    if (typeof chrome !== "undefined" && chrome.windows && chrome.windows.getAll) {
      const windows = await chrome.windows.getAll({ populate: true });
      for (const win of windows) {
        if (win.type === "popup" && win.tabs) {
          for (const tab of win.tabs) {
            if (tab.url && tab.url.includes("popup.html")) {
              console.log("[AlgoDeck] Zamknięto stare okno popup przywrócone z poprzedniej sesji:", win.id);
              chrome.windows.remove(win.id).catch(() => {});
              break;
            }
          }
        }
      }
    }
  } catch (_) {}

  // Wyczyść ewentualne stare klucze powiadomień w storage
  try {
    if (typeof chrome !== "undefined" && chrome.storage && chrome.storage.local) {
      chrome.storage.local.remove(["auto_pdf", "auto_pdf_ts"]);
    }
  } catch (_) {}
}

cleanupRestoredWindows();

// ------------------------------------------------------------------------------
// INDEKSOWANIE HISTORII POBRAŃ PRZEGLĄDARKI (Wyciszenie starych plików)
// ------------------------------------------------------------------------------
async function initSessionIndex() {
  try {
    // 1. Wczytaj zapamiętane pobrania z lokalnego storage
    if (typeof chrome !== "undefined" && chrome.storage && chrome.storage.local) {
      try {
        const stored = await chrome.storage.local.get(["seen_files", "seen_downloads"]);
        if (stored.seen_files && typeof stored.seen_files === "object") {
          seenFilesMap = stored.seen_files;
        }
        if (Array.isArray(stored.seen_downloads)) {
          for (const id of stored.seen_downloads) {
            handledDownloads.add(id);
          }
        }
      } catch (_) {}
    }

    // 2. Pobierz wszystkie istniejące pliki z bazy pobrań przeglądarki i zablokuj je
    if (typeof chrome !== "undefined" && chrome.downloads && chrome.downloads.search) {
      try {
        const existing = await chrome.downloads.search({});
        for (const item of existing) {
          if (item.id) handledDownloads.add(item.id);
          if (item.filename) {
            const normKey = item.filename.toLowerCase();
            seenFilesMap[normKey] = {
              downloadId: item.id,
              handled: true,
              time: item.startTime ? new Date(item.startTime).getTime() : 0
            };
          }
        }
      } catch (_) {}
    }

    // 3. Zapisz zindeksowany stan
    if (typeof chrome !== "undefined" && chrome.storage && chrome.storage.local) {
      try {
        const entries = Object.entries(seenFilesMap).slice(-500);
        seenFilesMap = Object.fromEntries(entries);
        await chrome.storage.local.set({
          seen_downloads: Array.from(handledDownloads).slice(-500),
          seen_files: seenFilesMap
        });
      } catch (_) {}
    }
  } catch (err) {
    console.warn("[AlgoDeck] Błąd indeksowania historii pobrań:", err);
  }
}

initSessionIndex();

async function markHandled(downloadId, filename, url) {
  handledDownloads.add(downloadId);
  liveDownloads.delete(downloadId);

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
      const dList = Array.from(handledDownloads).slice(-500);
      const entries = Object.entries(seenFilesMap).slice(-500);
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

// ------------------------------------------------------------------------------
// PRZETWARZANIE POBRANEGO PLIKU
// ------------------------------------------------------------------------------
async function processDownload(downloadId) {
  // Jeśli startup mute nadal trwa, porzuć
  if (Date.now() - SW_BOOT_TIME < STARTUP_MUTE_MS) {
    handledDownloads.add(downloadId);
    return;
  }

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

    // 2. Walidacja stanu pobierania
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

    // 3. Sprawdź ustawienia auto_detect
    try {
      const stored = await chrome.storage.local.get(["auto_detect_downloads"]);
      if (stored.auto_detect_downloads === false) {
        markHandled(downloadId, filename, item.url);
        return;
      }
    } catch (_) {}

    // 4. Sprawdź czy zadanie już istnieje w workspace
    const alreadyExists = await isProblemAlreadyInWorkspace(filename);
    if (alreadyExists) {
      console.log("[AlgoDeck] Zadanie z pliku", filename, "już istnieje w workspace. Pomijam powiadomienie.");
      markHandled(downloadId, filename, item.url);
      return;
    }

    if (handledDownloads.has(downloadId) || seenFilesMap[normKey]) return;
    await markHandled(downloadId, filename, item.url);

    console.log("[AlgoDeck] Wykryto ŚWIEŻE pobranie zadania:", filename);

    // Zapisz w storage rozszerzenia
    try {
      await chrome.storage.local.set({
        auto_pdf: filename,
        auto_pdf_ts: Date.now(),
        auto_is_zip: filename.toLowerCase().endsWith(".zip")
      });
    } catch (_) {}

    // Odznaka na ikonie rozszerzenia
    try {
      if (chrome.action && chrome.action.setBadgeText) {
        chrome.action.setBadgeText({ text: "1" });
        chrome.action.setBadgeBackgroundColor({ color: "#10b981" });
      }
    } catch (_) {}

    // Powiadomienie natywne
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

    // Otwórz DOKŁADNIE JEDNO dedykowane okno powiadomienia
    await openDedicatedNotificationWindow(filename);
  });
}

// ------------------------------------------------------------------------------
// PANCERNY SINGLETON OKNA POWIADOMIENIA (Max 1 okno, Cooldown 10s, Zero spamu)
// ------------------------------------------------------------------------------
async function openDedicatedNotificationWindow(filename) {
  if (!filename) return;

  const now = Date.now();

  // 1. Blokada startowa
  if (now - SW_BOOT_TIME < STARTUP_MUTE_MS) {
    console.log("[AlgoDeck] Startup mute aktywny - okno nie zostanie utworzone.");
    return;
  }

  const targetUrl = chrome.runtime.getURL(`popup.html?auto_pdf=${encodeURIComponent(filename)}`);

  // 2. Jeśli mamy zapamiętane aktywne okno, zaktualizuj je i wysuń na wierzch
  if (activeNotificationWindowId !== null) {
    try {
      const win = await chrome.windows.get(activeNotificationWindowId, { populate: true });
      if (win) {
        if (win.tabs && win.tabs.length > 0) {
          await chrome.tabs.update(win.tabs[0].id, { url: targetUrl });
        }
        await chrome.windows.update(activeNotificationWindowId, { focused: true });
        lastWindowOpenedTime = now;
        return;
      }
    } catch (_) {
      activeNotificationWindowId = null;
    }
  }

  // 3. Sprawdź czy JAKIEKOLWIEK inne okno popup z popup.html już istnieje w przeglądarce
  try {
    const allWins = await chrome.windows.getAll({ populate: true });
    for (const win of allWins) {
      if (win.type === "popup" && win.tabs) {
        for (const tab of win.tabs) {
          if (tab.url && tab.url.includes("popup.html")) {
            activeNotificationWindowId = win.id;
            await chrome.tabs.update(tab.id, { url: targetUrl });
            await chrome.windows.update(win.id, { focused: true });
            lastWindowOpenedTime = now;
            return;
          }
        }
      }
    }
  } catch (_) {}

  // 4. Cooldown: Nie pozwól na utworzenie nowego okna częściej niż raz na WINDOW_COOLDOWN_MS
  if (now - lastWindowOpenedTime < WINDOW_COOLDOWN_MS) {
    console.log("[AlgoDeck] Window cooldown aktywny (odstęp < 10s) - pomijam tworzenie kolejnego okna.");
    return;
  }

  // 5. Mutex współbieżności
  if (isOpeningWindow) {
    console.log("[AlgoDeck] Blokada współbieżności - tworzenie okna w toku.");
    return;
  }
  isOpeningWindow = true;

  try {
    const createdWin = await chrome.windows.create({
      url: targetUrl,
      type: "popup",
      width: 390,
      height: 155,
      focused: true
    });
    if (createdWin && createdWin.id) {
      activeNotificationWindowId = createdWin.id;
      lastWindowOpenedTime = Date.now();
    }
  } catch (err) {
    console.warn("[AlgoDeck] Błąd chrome.windows.create:", err);
  } finally {
    isOpeningWindow = false;
  }
}

// Czyszczenie referencji okna po jego zamknięciu
if (typeof chrome !== "undefined" && chrome.windows && chrome.windows.onRemoved) {
  chrome.windows.onRemoved.addListener((windowId) => {
    if (windowId === activeNotificationWindowId) {
      activeNotificationWindowId = null;
    }
  });
}

// Kliknięcie w powiadomienie Chrome
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
// NASŁUCHIWANIE POBRAŃ — BEZWZGLĘDNE ODRZUCANIE HISTORII I STARTUP MUTE
// ------------------------------------------------------------------------------

// onCreated: rejestrujemy WYŁĄCZNIE pobrania rozpoczęte na żywo po upływie STARTUP_MUTE_MS
if (typeof chrome !== "undefined" && chrome.downloads && chrome.downloads.onCreated) {
  chrome.downloads.onCreated.addListener((item) => {
    if (!item || !item.id) return;

    // A. Startup mute: pierwsze 10 sekund po uruchomieniu ignorujemy wszystko
    if (Date.now() - SW_BOOT_TIME < STARTUP_MUTE_MS) {
      handledDownloads.add(item.id);
      return;
    }

    // B. Tylko stan in_progress może być nowym pobraniem na żywo
    if (item.state !== "in_progress") {
      handledDownloads.add(item.id);
      return;
    }

    // C. Walidacja czasu startu (jeśli jest podany)
    if (item.startTime) {
      const st = new Date(item.startTime).getTime();
      if (!isNaN(st) && st < SW_BOOT_TIME) {
        handledDownloads.add(item.id);
        return;
      }
    }

    // D. Sprawdź czy plik o tej nazwie był już kiedykolwiek obsłużony
    if (item.filename) {
      const norm = item.filename.toLowerCase();
      if (seenFilesMap[norm]) {
        handledDownloads.add(item.id);
        return;
      }
    }

    // Rejestrujemy to pobranie jako świeże pobranie na żywo
    liveDownloads.set(item.id, Date.now());
  });
}

// onChanged: reagujemy TYLKO na zakończenie pobrań obecnych w liveDownloads
if (typeof chrome !== "undefined" && chrome.downloads && chrome.downloads.onChanged) {
  chrome.downloads.onChanged.addListener((delta) => {
    if (!delta || !delta.id) return;

    // A. Jeśli trwa startup mute - ignoruj
    if (Date.now() - SW_BOOT_TIME < STARTUP_MUTE_MS) {
      handledDownloads.add(delta.id);
      liveDownloads.delete(delta.id);
      return;
    }

    // B. Pobranie MUSI być obecne w liveDownloads (czyli rozpoczęte na żywo po mute)
    if (!liveDownloads.has(delta.id)) {
      handledDownloads.add(delta.id);
      return;
    }

    // C. Reaguj tylko na zakończenie
    if (delta.state && delta.state.current === "complete") {
      const startedAt = liveDownloads.get(delta.id);
      liveDownloads.delete(delta.id);

      // Jeśli pobieranie trwało ponad 10 minut, nie pokazuj okna
      if (Date.now() - startedAt > 600000) {
        handledDownloads.add(delta.id);
        return;
      }

      processDownload(delta.id);
    }
  });
}
