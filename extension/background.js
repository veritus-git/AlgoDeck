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
// WERYFIKACJA STANU USŁUGI ALGODECK
// Gdy AlgoDeck jest wyłączony (offline), autowykrywanie NIE MOŻE działać!
// ------------------------------------------------------------------------------
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

// ------------------------------------------------------------------------------
// DODAWANIE POBRANEGO PLIKU DO KOLEJKI (STAGING) W TLE
// Całkowity brak samoczynnego otwierania okien/popupów.
// Plik jest zaciągany w tle i czeka w poczekalni rozszerzenia.
// ------------------------------------------------------------------------------
async function triggerDetectedAction(filename) {
  if (!filename) return;

  const isZip = filename.toLowerCase().endsWith(".zip");
  const isPdf = filename.toLowerCase().endsWith(".pdf");

  // 1. Zapisz w pamięci lokalnej rozszerzenia ostatnio pobrany plik
  try {
    const dataToSave = {};
    if (isPdf) {
      dataToSave.last_downloaded_pdf = filename;
      dataToSave.last_pdf_ts = Date.now();
    } else if (isZip) {
      dataToSave.last_downloaded_zip = filename;
      dataToSave.last_zip_ts = Date.now();
    }
    await chrome.storage.local.set(dataToSave);
  } catch (_) {}

  // 2. Jeśli usługa AlgoDeck działa, zaciągnij plik bezpośrednio do stagingu w tle!
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
    console.warn("[AlgoDeck] Błąd przesyłania ścieżki do stagingu:", err);
  }

  // 3. Ustaw zieloną odznakę "1" na ikonie paska rozszerzenia
  try {
    if (chrome.action && chrome.action.setBadgeText) {
      chrome.action.setBadgeText({ text: "1" });
      chrome.action.setBadgeBackgroundColor({ color: "#10b981" });
    }
  } catch (_) {}
}

// ------------------------------------------------------------------------------
// PRZETWARZANIE POBRANEGO PLIKU
// ------------------------------------------------------------------------------
async function processDownload(downloadId) {
  if (handledDownloads.has(downloadId)) return;

  // 1. BEZWZGLĘDNE SPRAWDZENIE: Gdy AlgoDeck jest wyłączony, autodetekcja NIE DZIAŁA!
  const isOnline = await isDaemonRunning();
  if (!isOnline) {
    console.log("[AlgoDeck] Usługa AlgoDeck jest wyłączona (offline). Ignoruję pobranie:", downloadId);
    markHandled(downloadId, null, null);
    return;
  }

  chrome.downloads.search({ id: downloadId }, async (items) => {
    if (!items || items.length === 0) return;
    const item = items[0];

    const filename = item.filename || "";

    // Ponowne sprawdzenie stanu procesu
    const stillOnline = await isDaemonRunning();
    if (!stillOnline) {
      markHandled(downloadId, filename, item.url);
      return;
    }

    // 2. Sprawdź czy pobieranie jest kompletne
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

    // 3. Sprawdź czy automatyczne wykrywanie nie zostało wyłączone w opcjach
    try {
      const stored = await chrome.storage.local.get(["auto_detect_downloads"]);
      if (stored.auto_detect_downloads === false) {
        markHandled(downloadId, filename, item.url);
        return;
      }
    } catch (_) {}

    await markHandled(downloadId, filename, item.url);

    console.log("[AlgoDeck] Pomyślnie wykryto nowe pobranie zadania:", filename);

    // Powiadomienie natywne
    const baseName = filename.replace(/^.*[\\\/]/, '');
    try {
      if (chrome.notifications && chrome.notifications.create) {
        chrome.notifications.create(`algodeck_${downloadId}`, {
          type: "basic",
          iconUrl: "icons/icon128.png",
          title: "AlgoDeck ⚡ Wykryto zadanie olimpijskie",
          message: `Pobrano: ${baseName}\nKliknij podmenu ⚡ na pasku, aby utworzyć zadanie.`,
          priority: 2,
          requireInteraction: false
        });
      }
    } catch (_) {}

    // Otwórz podmenu rozszerzenia (bez tworzenia żadnych okien systemu operacyjnego)
    await triggerDetectedAction(filename);
  });
}

// Kliknięcie w powiadomienie Chrome
if (typeof chrome !== "undefined" && chrome.notifications && chrome.notifications.onClicked) {
  chrome.notifications.onClicked.addListener(async (notifId) => {
    if (notifId && notifId.startsWith("algodeck_")) {
      chrome.notifications.clear(notifId);
      
      const isOnline = await isDaemonRunning();
      if (!isOnline) return;

      try {
        if (chrome.action && typeof chrome.action.openPopup === "function") {
          await chrome.action.openPopup();
        }
      } catch (_) {}
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
// NASŁUCHIWANIE POBRAŃ — TYLKO NOWE POBRANIA NA ŻYWO GDY ALGODECK JEST ONLINE
// ------------------------------------------------------------------------------

if (typeof chrome !== "undefined" && chrome.downloads && chrome.downloads.onCreated) {
  chrome.downloads.onCreated.addListener(async (item) => {
    if (!item || !item.id) return;

    // Pobranie z historii przy starcie nie jest "in_progress"
    if (item.state !== "in_progress") {
      handledDownloads.add(item.id);
      return;
    }

    if (handledDownloads.has(item.id)) return;

    // Gdy AlgoDeck jest wyłączony, nie rejestruj pobrania
    const isOnline = await isDaemonRunning();
    if (!isOnline) {
      handledDownloads.add(item.id);
      return;
    }

    // Rejestrujemy to pobranie jako świeże pobranie na żywo
    liveDownloads.set(item.id, Date.now());
  });
}

if (typeof chrome !== "undefined" && chrome.downloads && chrome.downloads.onChanged) {
  chrome.downloads.onChanged.addListener(async (delta) => {
    if (!delta || !delta.id) return;

    // Pobranie MUSI być obecne w liveDownloads (czyli zarejestrowane na żywo w tej sesji)
    if (!liveDownloads.has(delta.id)) {
      handledDownloads.add(delta.id);
      return;
    }

    if (delta.state && delta.state.current === "complete") {
      liveDownloads.delete(delta.id);
      handledDownloads.add(delta.id);

      const isOnline = await isDaemonRunning();
      if (!isOnline) return;

      processDownload(delta.id);
    }
  });
}
