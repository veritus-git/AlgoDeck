// AlgoDeck ⚡ Koło MAP - Background Service Worker
// Niezawodne, rygorystyczne wykrywanie ŚWIEŻO pobranych zadań PDF i ZIP (Szkopuł, Koło MAP, OIJ, OI)
// - Wykorzystuje znacznik czasu t=Date.now() w URL powiadomienia, dzięki czemu stare okna przywrócone przez sesję przeglądarki same się natychmiast zamykają
// - Agresywne czyszczenie zombi-okien przywróconych z poprzedniej sesji przeglądarki (Session Restore)
// - Twardy singleton okna powiadomienia: w systemie może istnieć MAKSYMALNIE 1 okienko powiadomienia na raz
// - Wykrywanie pobrań nie blokuje zadań, które już są w workspace (użytkownik może pobrać zadanie ponownie i natychmiast je otworzyć)

const API_BASE = "http://127.0.0.1:8080";

// Rejestry pobrań
const liveDownloads = new Map(); // downloadId -> timestamp rejestracji na żywo
const handledDownloads = new Set();
let seenFilesMap = {};

let isOpeningWindow = false;
let activeNotificationWindowId = null;

// ------------------------------------------------------------------------------
// SPRZĄTANIE PRZYWRÓCONYCH ZOMBI-OKIEN SESJI (Brave/Chrome Session Restore)
// ------------------------------------------------------------------------------
async function cleanupRestoredWindows() {
  try {
    if (typeof chrome !== "undefined" && chrome.windows && chrome.windows.getAll) {
      const windows = await chrome.windows.getAll({ populate: true });
      for (const win of windows) {
        for (const tab of win.tabs || []) {
          // Jeśli karta to popup.html (zwłaszcza z auto_pdf) w osobnym oknie
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

  // Wyczyść ewentualne stare klucze powiadomień w storage po restarcie
  try {
    if (typeof chrome !== "undefined" && chrome.storage && chrome.storage.local) {
      chrome.storage.local.remove(["auto_pdf", "auto_pdf_ts"]);
    }
  } catch (_) {}
}

cleanupRestoredWindows();

// ------------------------------------------------------------------------------
// INDEKSOWANIE HISTORII POBRAŃ PRZEGLĄDARKI (Odrzucanie starych plików)
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

    // 2. Pobierz wszystkie istniejące wpisy z historii przeglądarki i oznacz je jako obsłużone
    if (typeof chrome !== "undefined" && chrome.downloads && chrome.downloads.search) {
      try {
        const existing = await chrome.downloads.search({});
        for (const item of existing) {
          if (item.id) handledDownloads.add(item.id);
          if (item.filename) {
            seenFilesMap[item.filename.toLowerCase()] = {
              downloadId: item.id,
              handled: true,
              time: item.startTime ? new Date(item.startTime).getTime() : 0
            };
          }
        }
      } catch (_) {}
    }

    // 3. Zapisz zaktualizowany indeks
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
    seenFilesMap[filename.toLowerCase()] = {
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

// ------------------------------------------------------------------------------
// PANCERNY SINGLETON OKNA POWIADOMIENIA (MAKSYMALNIE 1 OKNO NA RAZ)
// ------------------------------------------------------------------------------
async function openDedicatedNotificationWindow(filename) {
  if (!filename) return;

  const targetUrl = chrome.runtime.getURL(`popup.html?auto_pdf=${encodeURIComponent(filename)}&t=${Date.now()}`);

  // 1. Jeśli mamy zapamiętane aktywne okno powiadomienia, zaktualizuj je i wysuń na wierzch
  if (activeNotificationWindowId !== null) {
    try {
      const win = await chrome.windows.get(activeNotificationWindowId, { populate: true });
      if (win) {
        if (win.tabs && win.tabs.length > 0) {
          await chrome.tabs.update(win.tabs[0].id, { url: targetUrl });
        }
        await chrome.windows.update(activeNotificationWindowId, { focused: true });
        return;
      }
    } catch (_) {
      activeNotificationWindowId = null;
    }
  }

  // 2. Sprawdź czy JAKIEKOLWIEK inne okno z popup.html istnieje w przeglądarce
  try {
    const allWins = await chrome.windows.getAll({ populate: true });
    for (const win of allWins) {
      for (const tab of win.tabs || []) {
        if (tab.url && tab.url.includes("popup.html")) {
          activeNotificationWindowId = win.id;
          await chrome.tabs.update(tab.id, { url: targetUrl });
          await chrome.windows.update(win.id, { focused: true });
          return;
        }
      }
    }
  } catch (_) {}

  // 3. Mutex blokujący wyścigi współbieżne
  if (isOpeningWindow) return;
  isOpeningWindow = true;

  try {
    const createdWin = await chrome.windows.create({
      url: targetUrl,
      type: "popup",
      width: 390,
      height: 140,
      focused: true
    });
    if (createdWin && createdWin.id) {
      activeNotificationWindowId = createdWin.id;
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

// ------------------------------------------------------------------------------
// PRZETWARZANIE POBRANEGO PLIKU
// ------------------------------------------------------------------------------
async function processDownload(downloadId) {
  if (handledDownloads.has(downloadId)) return;

  chrome.downloads.search({ id: downloadId }, async (items) => {
    if (!items || items.length === 0) return;
    const item = items[0];

    const filename = item.filename || "";
    const normKey = filename.toLowerCase();

    // 1. Sprawdź czy pobieranie jest kompletne
    if (item.state !== "complete") {
      setTimeout(() => {
        if (!handledDownloads.has(downloadId)) {
          processDownload(downloadId);
        }
      }, 400);
      return;
    }

    if (filename.endsWith(".crdownload") || filename.endsWith(".tmp")) {
      setTimeout(() => processDownload(downloadId), 300);
      return;
    }

    if (!isOlympicTaskFile(item)) {
      markHandled(downloadId, filename, item.url);
      return;
    }

    // 2. Sprawdź czy automatyczne wykrywanie nie zostało wyłączone w opcjach
    try {
      const stored = await chrome.storage.local.get(["auto_detect_downloads"]);
      if (stored.auto_detect_downloads === false) {
        markHandled(downloadId, filename, item.url);
        return;
      }
    } catch (_) {}

    await markHandled(downloadId, filename, item.url);

    console.log("[AlgoDeck] Pomyślnie wykryto nowe pobranie zadania:", filename);

    // Zapisz w storage rozszerzenia dla popupa (dostępne przez 120s)
    try {
      await chrome.storage.local.set({
        auto_pdf: filename,
        auto_pdf_ts: Date.now(),
        auto_is_zip: filename.toLowerCase().endsWith(".zip")
      });
    } catch (_) {}

    // Zielona odznaka na ikonie rozszerzenia na pasku zadań
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

    // Otwórz DOKŁADNIE JEDNO okienko 1-kliknięcia
    await openDedicatedNotificationWindow(filename);
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
// NASŁUCHIWANIE POBRAŃ — TYLKO NOWE POBRANIA NA ŻYWO W TRAKCIE SESJI
// ------------------------------------------------------------------------------

if (typeof chrome !== "undefined" && chrome.downloads && chrome.downloads.onCreated) {
  chrome.downloads.onCreated.addListener((item) => {
    if (!item || !item.id) return;

    // Pobranie z historii przy starcie nie jest "in_progress"
    if (item.state !== "in_progress") {
      handledDownloads.add(item.id);
      return;
    }

    if (handledDownloads.has(item.id)) return;

    // Rejestrujemy to pobranie jako świeże pobranie na żywo
    liveDownloads.set(item.id, Date.now());
  });
}

if (typeof chrome !== "undefined" && chrome.downloads && chrome.downloads.onChanged) {
  chrome.downloads.onChanged.addListener((delta) => {
    if (!delta || !delta.id) return;

    // Pobranie MUSI być obecne w liveDownloads (czyli rozpoczęte na żywo w tej sesji)
    if (!liveDownloads.has(delta.id)) {
      handledDownloads.add(delta.id);
      return;
    }

    if (delta.state && delta.state.current === "complete") {
      liveDownloads.delete(delta.id);
      handledDownloads.add(delta.id);
      processDownload(delta.id);
    }
  });
}
