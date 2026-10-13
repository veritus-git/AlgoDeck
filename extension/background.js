// AlgoDeck ⚡ Koło MAP - Background Service Worker
// Niezawodne, rygorystyczne wykrywanie ŚWIEŻO pobranych zadań PDF i ZIP (Szkopuł, Koło MAP, OIJ, OI)
// - Wykorzystuje chrome.storage.session do jednoznacznej identyfikacji startu przeglądarki
// - Chroni przed powielaniem okienek po restarcie komputera (stare pobrania są w 100% ignorowane)
// - Obsługuje wyłącznie pobrania zainicjowane W TEJ SESJI przez użytkownika
// - Singleton lock na otwieranie dedykowanego okna (dokładnie 1 instancja powiadomienia)

const API_BASE = "http://127.0.0.1:8080";

let sessionBootEpoch = 0;
let isSessionReady = false;
let initPromise = null;

// Pobrania zainicjowane (onCreated) WŁAŚNIE W TEJ SESJI po inicjalizacji
const sessionCreatedDownloads = new Set();
const handledDownloads = new Set();
let seenFilesMap = {};

let activeNotificationWindowId = null;
let isOpeningWindow = false;

// 1. Inicjalizacja sesji z gwarancją odróżnienia restartu komputera / przeglądarki
async function ensureSessionInitialized() {
  if (isSessionReady) return;
  if (initPromise) return initPromise;

  initPromise = (async () => {
    try {
      // chrome.storage.session czyści się samoczynnie przy restarcie przeglądarki!
      let bootTime = 0;
      try {
        if (typeof chrome !== "undefined" && chrome.storage && chrome.storage.session) {
          const sess = await chrome.storage.session.get(["session_boot_time"]);
          if (sess && sess.session_boot_time) {
            bootTime = sess.session_boot_time;
          }
        }
      } catch (e) {
        console.warn("[AlgoDeck] storage.session niedostępny:", e);
      }

      if (!bootTime) {
        // ŚWIEŻY START PRZEGLĄDARKI / RESTART KOMPUTERA
        bootTime = Date.now();
        sessionBootEpoch = bootTime;
        try {
          if (typeof chrome !== "undefined" && chrome.storage && chrome.storage.session) {
            await chrome.storage.session.set({ session_boot_time: bootTime });
          }
        } catch (_) {}

        // Wyczyść stare, wiszące stany z poprzednich dni / sesji
        try {
          if (typeof chrome !== "undefined" && chrome.storage && chrome.storage.local) {
            await chrome.storage.local.remove([
              "auto_pdf", "auto_pdf_ts", "auto_is_zip",
              "algodeck_staged_pdf", "algodeck_staged_zip"
            ]);
          }
        } catch (_) {}
      } else {
        sessionBootEpoch = bootTime;
      }

      // Wczytaj zindeksowane wcześniej pliki z local storage
      if (typeof chrome !== "undefined" && chrome.storage && chrome.storage.local) {
        const local = await chrome.storage.local.get(["seen_files", "seen_downloads"]);
        if (local.seen_files && typeof local.seen_files === "object") {
          seenFilesMap = local.seen_files;
        }
        if (Array.isArray(local.seen_downloads)) {
          for (const id of local.seen_downloads) {
            handledDownloads.add(id);
          }
        }
      }

      // Zindeksuj WSZYSTKIE istniejące już pobrania w historii przeglądarki
      // Żadne z nich nie może być traktowane jako nowe!
      if (typeof chrome !== "undefined" && chrome.downloads && chrome.downloads.search) {
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
      }

      // Zapisz zaktualizowaną historię
      if (typeof chrome !== "undefined" && chrome.storage && chrome.storage.local) {
        const entries = Object.entries(seenFilesMap).slice(-300);
        seenFilesMap = Object.fromEntries(entries);
        await chrome.storage.local.set({
          seen_downloads: Array.from(handledDownloads).slice(-200),
          seen_files: seenFilesMap
        });
      }
    } catch (err) {
      console.warn("[AlgoDeck] Błąd inicjalizacji sesji pobrań:", err);
    } finally {
      isSessionReady = true;
    }
  })();

  return initPromise;
}

// Uruchom od razu
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
      const dList = Array.from(handledDownloads).slice(-200);
      const entries = Object.entries(seenFilesMap).slice(-300);
      seenFilesMap = Object.fromEntries(entries);
      await chrome.storage.local.set({
        seen_downloads: dList,
        seen_files: seenFilesMap
      });
    }
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

    // 2. Dodatkowa walidacja czasu: pobranie MUSIAŁO się rozpocząć po czasie startu sesji (+ margines)
    const startTime = item.startTime ? new Date(item.startTime).getTime() : 0;
    if (startTime > 0 && startTime < sessionBootEpoch) {
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
      markHandled(downloadId, filename, item.url);
      return;
    }

    // 4. Sprawdź czy automatyczne wykrywanie nie zostało wyłączone w opcjach
    let autoDetect = true;
    try {
      const stored = await chrome.storage.local.get(["auto_detect_downloads"]);
      if (stored.auto_detect_downloads === false) {
        autoDetect = false;
      }
    } catch (e) {}

    if (!autoDetect) {
      markHandled(downloadId, filename, item.url);
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

    console.log("[AlgoDeck] Rzeczywiście wykryto NOWO pobrany plik zadania:", filename);

    // Zapisz w pamięci podręcznej rozszerzenia
    try {
      await chrome.storage.local.set({
        auto_pdf: filename,
        auto_pdf_ts: Date.now(),
        auto_is_zip: filename.toLowerCase().endsWith(".zip")
      });
    } catch (e) {}

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

    // Otwórz DOKŁADNIE JEDNO dedykowane okienko powiadomienia (singleton lock)
    await openDedicatedNotificationWindow(filename);
  });
}

// Otwieranie dedykowanego okienka powiadomienia (Singleton Lock - ZAWSZE maksymalnie 1 okno!)
async function openDedicatedNotificationWindow(filename) {
  if (isOpeningWindow) {
    console.log("[AlgoDeck] Okno powiadomienia jest już w trakcie otwierania - pomijam duplikat.");
    return;
  }
  isOpeningWindow = true;

  try {
    const targetUrl = chrome.runtime.getURL(`popup.html?auto_pdf=${encodeURIComponent(filename)}`);

    // Przeszukaj wszystkie istniejące okna przeglądarki pod kątem otwartego już popup.html AlgoDeck
    try {
      const allWindows = await chrome.windows.getAll({ populate: true });
      for (const win of allWindows) {
        if (win.tabs) {
          const popupTab = win.tabs.find(t => t.url && t.url.includes("popup.html"));
          if (popupTab) {
            await chrome.tabs.update(popupTab.id, { url: targetUrl });
            await chrome.windows.update(win.id, { focused: true });
            activeNotificationWindowId = win.id;
            return;
          }
        }
      }
    } catch (_) {}

    // Jeśli nie ma otwartego okna AlgoDeck, stwórz DOKŁADNIE JEDNO
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
  } finally {
    setTimeout(() => {
      isOpeningWindow = false;
    }, 600);
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

// Nasłuchiwanie utworzenia nowego pobrania (onCreated)
if (typeof chrome !== "undefined" && chrome.downloads && chrome.downloads.onCreated) {
  chrome.downloads.onCreated.addListener(async (item) => {
    await ensureSessionInitialized();

    if (!item || !item.id) return;

    // Ignoruj pliki, które powstały przed startem tej sesji przeglądarki
    const itemStart = item.startTime ? new Date(item.startTime).getTime() : Date.now();
    if (itemStart < sessionBootEpoch) {
      handledDownloads.add(item.id);
      return;
    }

    // Rejestrujemy, że ten plik został zainicjowany W TEJ SESJI przez użytkownika!
    sessionCreatedDownloads.add(item.id);

    if (item.state === "complete") {
      processDownload(item.id);
    }
  });
}

// Nasłuchiwanie zmian stanu pobierania (onChanged)
if (typeof chrome !== "undefined" && chrome.downloads && chrome.downloads.onChanged) {
  chrome.downloads.onChanged.addListener(async (delta) => {
    await ensureSessionInitialized();

    if (!delta || !delta.id) return;

    // KLUCZOWE ZABEZPIECZENIE:
    // Jeśli pobranie NIE zostało utworzone w tej aktywnej sesji przez użytkownika,
    // to jest to zdarzenie odtwarzania historii przez przeglądarkę po restarcie — ignorujemy!
    if (!sessionCreatedDownloads.has(delta.id)) {
      handledDownloads.add(delta.id);
      return;
    }

    if (delta.state && delta.state.current === "complete") {
      processDownload(delta.id);
    } else if (delta.filename && !delta.filename.current.endsWith(".crdownload")) {
      processDownload(delta.id);
    }
  });
}

// Kliknięcie w powiadomienie systemowe
if (typeof chrome !== "undefined" && chrome.notifications && chrome.notifications.onClicked) {
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
