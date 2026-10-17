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
            if (win.type === "popup" || tab.url.includes("oneclick=") || tab.url.includes("auto_pdf=")) {
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
  let filename = (item.filename || "").toLowerCase();
  if (filename.endsWith(".crdownload")) {
    filename = filename.slice(0, -11);
  }
  const url = (item.url || "").toLowerCase();
  const finalUrl = (item.finalUrl || "").toLowerCase();
  const mime = (item.mime || "").toLowerCase();

  // 1. Nazwa pliku kończy się na .pdf
  if (filename.endsWith(".pdf")) return true;

  // 2. MIME type wskazuje na PDF
  if (mime.includes("pdf")) return true;

  // 3. Adres URL lub finalUrl zawiera słowo kluczowe związane z PDF
  if (url.includes(".pdf") || url.includes("/pdf") || url.includes("task/pdf") || url.includes("statement")) return true;
  if (finalUrl.includes(".pdf") || finalUrl.includes("/pdf") || finalUrl.includes("task/pdf") || finalUrl.includes("statement")) return true;

  // 4. Jeśli pobieranie pochodzi z platform olimpijskich (Szkopuł, SIO2, OIOIOI, OI) i nie jest kodem ani zipem
  const isOlympicDomain = url.includes("szkopul.edu.pl") || url.includes("oioioi") || url.includes("sio2") || url.includes("oi.edu.pl");
  if (isOlympicDomain) {
    if (!filename.endsWith(".zip") && !filename.endsWith(".cpp") && !filename.endsWith(".tar.gz")) {
      return true;
    }
  }

  return false;
}

// Sprawdzenie czy usługa AlgoDeck działa
async function isDaemonRunning() {
  try {
    const controller = new AbortController();
    const timer = setTimeout(() => controller.abort(), 1500);
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

// Czyszczenie starego stanu po ponownym uruchomieniu przeglądarki
if (typeof chrome !== "undefined" && chrome.runtime && chrome.runtime.onStartup) {
  chrome.runtime.onStartup.addListener(async () => {
    try {
      console.log("[AlgoDeck] Start przeglądarki — czyszczenie starego stanu zadań.");
      await chrome.storage.local.remove(["detected_task", "dismiss_oneclick", "last_downloaded_pdf"]);
      if (chrome.action && chrome.action.setBadgeText) {
        chrome.action.setBadgeText({ text: "" });
      }
      await fetch(`${API_BASE}/api/clear-staged`, { method: "POST" }).catch(() => {});
    } catch (_) {}
  });
}

const processedDownloads = new Set();
const initialIgnoredIds = new Set();
const initialIgnoredFiles = new Set();
let algodeckInstalledAt = null;

async function initDownloadIgnoreList() {
  try {
    const stored = await chrome.storage.local.get([
      "algodeck_installed_at",
      "initial_ignored_ids",
      "initial_ignored_files"
    ]);

    if (!stored.algodeck_installed_at) {
      let installedAt = Date.now();
      try {
        const res = await fetch(`${API_BASE}/api/initial-downloads`);
        if (res.ok) {
          const data = await res.json();
          if (data.installed_at) installedAt = data.installed_at;
          if (Array.isArray(data.ignored_files)) {
            data.ignored_files.forEach(f => initialIgnoredFiles.add(f.toLowerCase()));
          }
        }
      } catch (_) {}

      const ignoredIds = [];
      if (typeof chrome !== "undefined" && chrome.downloads && chrome.downloads.search) {
        try {
          const allDownloads = await chrome.downloads.search({});
          for (const dl of allDownloads || []) {
            ignoredIds.push(dl.id);
            initialIgnoredIds.add(dl.id);
            if (dl.filename) {
              const baseName = dl.filename.replace(/^.*[\\\/]/, '').toLowerCase();
              initialIgnoredFiles.add(baseName);
            }
          }
        } catch (_) {}
      }

      algodeckInstalledAt = installedAt;
      await chrome.storage.local.set({
        algodeck_installed_at: installedAt,
        initial_ignored_ids: ignoredIds,
        initial_ignored_files: Array.from(initialIgnoredFiles)
      });
      console.log(`[AlgoDeck] Zainicjalizowano ignorowanie starych pobrań (${ignoredIds.length} plików).`);
    } else {
      algodeckInstalledAt = stored.algodeck_installed_at;
      if (Array.isArray(stored.initial_ignored_ids)) {
        stored.initial_ignored_ids.forEach(id => initialIgnoredIds.add(id));
      }
      if (Array.isArray(stored.initial_ignored_files)) {
        stored.initial_ignored_files.forEach(f => initialIgnoredFiles.add(f.toLowerCase()));
      }
    }
  } catch (e) {
    console.warn("[AlgoDeck] Błąd initDownloadIgnoreList:", e);
  }
}
initDownloadIgnoreList();

if (typeof chrome !== "undefined" && chrome.runtime && chrome.runtime.onInstalled) {
  chrome.runtime.onInstalled.addListener(() => {
    initDownloadIgnoreList();
  });
}

// Obsługa zakończonego pobierania (wyłącznie świeże PDFy pobrane w tej sesji!)
async function handleCompletedDownload(downloadId) {
  if (!downloadId) return;

  if (processedDownloads.has(downloadId) || initialIgnoredIds.has(downloadId)) {
    return;
  }

  try {
    // 1. Pobierz informacje o pliku z retry (Chrome na macOS potrzebuje chwili na sfinalizowanie pliku)
    let item = null;
    for (let i = 0; i < 20; i++) {
      const items = await chrome.downloads.search({ id: downloadId });
      if (items && items.length > 0) {
        item = items[0];
        const fn = (item.filename || "").toLowerCase();
        if (item.state === "complete" && fn && !fn.endsWith(".crdownload") && !fn.endsWith(".tmp")) {
          break;
        }
      }
      await new Promise(r => setTimeout(r, 150));
    }

    if (!item || item.state !== "complete") {
      // Plik jeszcze się nie sfinalizował — nie oznaczaj jako przetworzonego, poczekaj na stan complete
      return;
    }

    if (!isOlympicTaskFile(item)) {
      return;
    }

    // Sprawdź czy plik nie pochodzi sprzed instalacji AlgoDeck
    const startTs = item.startTime ? new Date(item.startTime).getTime() : 0;
    const finishTs = item.endTime ? new Date(item.endTime).getTime() : startTs;
    if (algodeckInstalledAt && Math.max(startTs, finishTs) <= algodeckInstalledAt) {
      console.log("[AlgoDeck] Pominięto plik sprzed instalacji AlgoDeck:", item.filename);
      initialIgnoredIds.add(downloadId);
      return;
    }

    let filename = item.filename || "";
    if (filename.endsWith(".crdownload")) {
      filename = filename.slice(0, -11);
    }
    if (!filename) return;

    const baseName = filename.replace(/^.*[\\\/]/, '');
    const lowerBase = baseName.toLowerCase();
    if (initialIgnoredFiles.has(lowerBase)) {
      if (algodeckInstalledAt && Math.max(startTs, finishTs) <= algodeckInstalledAt + 3000) {
        console.log("[AlgoDeck] Pominięto plik z początkowej listy pobranych:", baseName);
        initialIgnoredIds.add(downloadId);
        return;
      }
    }

    // Sprawdź czy automatyczne wykrywanie nie zostało wyłączone w opcjach
    try {
      const stored = await chrome.storage.local.get(["auto_detect_downloads"]);
      if (stored.auto_detect_downloads === false) {
        console.log("[AlgoDeck] Autowykrywanie wyłączone w opcjach.");
        return;
      }
    } catch (_) {}

    // Dopiero po pomyślnej weryfikacji oznacz jako przetworzony
    processedDownloads.add(downloadId);
    if (processedDownloads.size > 200) {
      const oldest = processedDownloads.values().next().value;
      processedDownloads.delete(oldest);
    }

    console.log("[AlgoDeck] Wykryto kompletne pobranie zadania PDF:", filename);

    // 2. Natychmiast zapisz w chrome.storage.local (żeby plik był od razu widoczny w popupie po kliknięciu!)
    await chrome.storage.local.set({
      last_downloaded_pdf: filename,
      last_pdf_name: baseName,
      last_pdf_ts: Date.now(),
      detected_task: {
        filename: baseName,
        code: baseName.replace(/\.pdf$/i, "").slice(0, 6).toLowerCase(),
        ts: Date.now()
      },
      dismiss_oneclick: false
    });
    await chrome.storage.local.remove(["cleared_pdf_name"]);

    // 3. Ustaw zieloną odznakę "1" na ikonie paska rozszerzenia
    try {
      if (chrome.action && chrome.action.setBadgeText) {
        chrome.action.setBadgeText({ text: "1" });
        chrome.action.setBadgeBackgroundColor({ color: "#10b981" });
      }
    } catch (_) {}

    // 4. Wyślij do Rust backendu (staging) lub Native Host
    let problemId = "";
    let stagedSuccessfully = false;
    try {
      const res = await fetch(`${API_BASE}/api/stage-by-path`, {
        method: "POST",
        headers: { "Content-Type": "application/json" },
        body: JSON.stringify({ file_path: filename })
      });
      if (res.ok) {
        const data = await res.json();
        problemId = data.problem_id || "";
        stagedSuccessfully = true;
        console.log("[AlgoDeck] Plik pomyślnie zaciągnięty do poczekalni w Rust:", data);
      }
    } catch (err) {
      console.warn("[AlgoDeck] Błąd przesyłania do /api/stage-by-path:", err);
    }

    if (!stagedSuccessfully) {
      try {
        if (typeof chrome !== "undefined" && chrome.runtime && chrome.runtime.sendNativeMessage) {
          chrome.runtime.sendNativeMessage("com.algodeck.launcher", { action: "stage_file", file_path: filename }, (resp) => {
            console.log("[AlgoDeck] Native Host staging fallback odpowiedź:", resp);
          });
        }
      } catch (_) {}
    }

    // 5. Zaktualizuj detected_task o kod zadania z analizy
    if (problemId) {
      await chrome.storage.local.set({
        detected_task: {
          filename: baseName,
          code: problemId,
          ts: Date.now()
        }
      });
    }

    // 6. Otwórz popup rozszerzenia pod ikoną ⚡ jeśli przeglądarka na to zezwala
    await triggerExtensionPopup(baseName, problemId);
  } catch (e) {
    console.warn("[AlgoDeck] Błąd handleCompletedDownload:", e);
  }
}

// Otwieranie natywnego popupu rozszerzenia pod ikoną ⚡ (wewnątrz przeglądarki)
async function triggerExtensionPopup(baseName, problemId) {
  try {
    if (typeof chrome !== "undefined" && chrome.action && chrome.action.openPopup) {
      try {
        const [activeTab] = await chrome.tabs.query({ active: true, lastFocusedWindow: true });
        if (activeTab && activeTab.windowId) {
          await chrome.action.openPopup({ windowId: activeTab.windowId });
        } else {
          await chrome.action.openPopup();
        }
        console.log("[AlgoDeck] Pomyślnie otworzono popup rozszerzenia pod ikoną ⚡.");
      } catch (openErr) {
        console.log("[AlgoDeck] chrome.action.openPopup oczekuje kliknięcia w ikonę ⚡ (zadanie czeka z odznaką 1):", openErr);
      }
    }
  } catch (err) {
    console.warn("[AlgoDeck] triggerExtensionPopup błąd:", err);
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
      if (!delta || !delta.id) return;
      const isComplete = delta.state && delta.state.current === "complete";
      if (isComplete) {
        handleCompletedDownload(delta.id);
      } else if (delta.filename && delta.filename.current) {
        const fn = delta.filename.current.toLowerCase();
        if (fn.endsWith(".pdf") && !fn.endsWith(".crdownload")) {
          // Na macOS zmiana nazwy z .crdownload na .pdf może nastąpić w osobnym zdarzeniu
          setTimeout(() => handleCompletedDownload(delta.id), 250);
        }
      }
    });
  }
}

// Sprawdź ostatnio pobrane pliki po starcie / obudzeniu service workera
async function checkRecentDownloadsOnWake() {
  try {
    if (typeof chrome !== "undefined" && chrome.downloads && chrome.downloads.search) {
      const recent = await chrome.downloads.search({
        limit: 5,
        orderBy: ["-startTime"],
        state: "complete"
      });
      for (const item of recent || []) {
        if (initialIgnoredIds.has(item.id)) continue;
        const finishTime = item.endTime ? new Date(item.endTime).getTime() : new Date(item.startTime).getTime();
        if (algodeckInstalledAt && finishTime <= algodeckInstalledAt) {
          initialIgnoredIds.add(item.id);
          continue;
        }
        const fn = item.filename || "";
        const baseName = fn.replace(/^.*[\\\/]/, '').toLowerCase();
        if (initialIgnoredFiles.has(baseName) && algodeckInstalledAt && finishTime <= algodeckInstalledAt + 3000) {
          initialIgnoredIds.add(item.id);
          continue;
        }
        if (isOlympicTaskFile(item)) {
          const age = Date.now() - finishTime;
          if (age < 180000) { // młodszy niż 3 minuty
            handleCompletedDownload(item.id);
            break;
          }
        }
      }
    }
  } catch (_) {}
}
checkRecentDownloadsOnWake();

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
