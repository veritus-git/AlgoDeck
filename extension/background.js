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
  const url = (item.url || "").toLowerCase().split("?")[0];
  const mime = (item.mime || "").toLowerCase();

  // Wyłącznie pliki .pdf! ZIPy nie są automatycznie zaciągane z pobrań
  return filename.endsWith(".pdf") || mime.includes("pdf") || url.endsWith(".pdf");
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

// Obsługa zakończonego pobierania (wyłącznie świeże PDFy pobrane w tej sesji!)
async function handleCompletedDownload(downloadId) {
  if (!downloadId) return;

  if (processedDownloads.has(downloadId)) {
    return;
  }
  processedDownloads.add(downloadId);
  if (processedDownloads.size > 200) {
    const oldest = processedDownloads.values().next().value;
    processedDownloads.delete(oldest);
  }

  try {
    // 1. Sprawdź czy usługa AlgoDeck działa
    const isOnline = await isDaemonRunning();
    if (!isOnline) {
      console.log("[AlgoDeck] Demon wyłączony — pomijam zadanie.");
      return;
    }

    // 2. Pobierz informacje o pliku z retry (Chrome potrzebuje chwili na usunięcie .crdownload)
    let item = null;
    for (let i = 0; i < 15; i++) {
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

    if (!item) return;

    // 3. Sprawdź czas rozpoczęcia: ignoruj stare pobrania z historii starsze niż 5 minut
    if (item.startTime) {
      const startTs = new Date(item.startTime).getTime();
      if (Date.now() - startTs > 300000) {
        console.log("[AlgoDeck] Zignorowano stare pobranie z historii:", item.filename, item.startTime);
        return;
      }
    }

    if (!isOlympicTaskFile(item)) {
      return;
    }

    // Sprawdź czy automatyczne wykrywanie nie zostało wyłączone w opcjach
    try {
      const stored = await chrome.storage.local.get(["auto_detect_downloads"]);
      if (stored.auto_detect_downloads === false) {
        console.log("[AlgoDeck] Autowykrywanie wyłączone w opcjach.");
        return;
      }
    } catch (_) {}

    let filename = item.filename || "";
    if (filename.endsWith(".crdownload")) {
      filename = filename.slice(0, -11);
    }
    if (!filename) return;

    const baseName = filename.replace(/^.*[\\\/]/, '');

    console.log("[AlgoDeck] Wykryto świeże pobranie zadania PDF:", filename);

    // 4. Zapisz w chrome.storage.local (tylko ten najnowszy plik)
    await chrome.storage.local.set({
      last_downloaded_pdf: filename,
      last_pdf_name: baseName,
      last_pdf_ts: Date.now()
    });
    await chrome.storage.local.remove(["cleared_pdf_name"]);

    // 4. Wyślij do Rust backendu (staging)
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
        console.log("[AlgoDeck] Plik pomyślnie zaciągnięty do poczekalni (staging) w Rust:", data);
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

    // 5. Ustaw zieloną odznakę "1" na ikonie paska rozszerzenia
    try {
      if (chrome.action && chrome.action.setBadgeText) {
        chrome.action.setBadgeText({ text: "1" });
        chrome.action.setBadgeBackgroundColor({ color: "#10b981" });
      }
    } catch (_) {}

    // 6. Otwórz popup rozszerzenia pod ikoną ⚡ na pasku przeglądarki (nie jako osobne okno systemu!)
    await triggerExtensionPopup(baseName, problemId);

    // 7. Powiadomienie systemowe
    try {
      if (chrome.notifications && chrome.notifications.create) {
        chrome.notifications.create(`algodeck_${downloadId}_${Date.now()}`, {
          type: "basic",
          iconUrl: "icons/icon128.png",
          title: "AlgoDeck ⚡ Nowe zadanie",
          message: `Pobrano: ${baseName}\nKliknij ikonę ⚡, aby utworzyć Workspace w VS Code.`,
          priority: 2,
          requireInteraction: false
        });
      }
    } catch (_) {}
  } catch (e) {
    console.warn("[AlgoDeck] Błąd handleCompletedDownload:", e);
  }
}

// Otwieranie natywnego popupu rozszerzenia pod ikoną ⚡ (wewnątrz przeglądarki, ZERO osobnych okien systemu)
async function triggerExtensionPopup(baseName, problemId) {
  try {
    await chrome.storage.local.set({
      detected_task: {
        filename: baseName,
        code: problemId,
        ts: Date.now()
      },
      dismiss_oneclick: false
    });

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
      const isRenamed = delta.filename && !delta.filename.current.endsWith(".crdownload");
      if (isComplete || isRenamed) {
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
