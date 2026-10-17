/* ==========================================================================
   AlgoDeck Extension - Client Logic
   Obsługa trybu offline (zielony przycisk), stagingu plików, zadań i Stream Decka
   ========================================================================== */

const API_BASE = "http://127.0.0.1:8080";

// Główne widoki
const viewOffline = document.getElementById("view-offline");
const viewDetected = document.getElementById("view-detected");
const viewMain = document.getElementById("view-main");

// Elementy offline
const btnStartAlgodeck = document.getElementById("btn-start-algodeck");
const offlineSpinner = document.getElementById("offline-spinner");
const offlineBtnText = document.getElementById("offline-btn-text");

// Zakładki
const tabButtons = document.querySelectorAll(".tab-btn");
const tabPanes = document.querySelectorAll(".tab-pane");

// Staging plików
const dropPdf = document.getElementById("drop-pdf");
const dropPdfBody = document.getElementById("drop-pdf-body");
const btnBrowsePdf = document.getElementById("btn-browse-pdf");
const fileInputPdf = document.getElementById("file-input-pdf");
const progressPdf = document.getElementById("progress-pdf");
const stagedPdfPill = document.getElementById("staged-pdf-pill");
const stagedPdfName = document.getElementById("staged-pdf-name");
const btnRemovePdf = document.getElementById("btn-remove-pdf");

const dropZip = document.getElementById("drop-zip");
const dropZipBody = document.getElementById("drop-zip-body");
const btnBrowseZip = document.getElementById("btn-browse-zip");
const fileInputZip = document.getElementById("file-input-zip");
const progressZip = document.getElementById("progress-zip");
const stagedZipPill = document.getElementById("staged-zip-pill");
const stagedZipName = document.getElementById("staged-zip-name");
const btnRemoveZip = document.getElementById("btn-remove-zip");

const btnSubmitFiles = document.getElementById("btn-submit-files");

// Formularz ręczny
const manCode = document.getElementById("man-code");
const manTitle = document.getElementById("man-title");
const manTime = document.getElementById("man-time");
const manMem = document.getElementById("man-mem");
const manCin = document.getElementById("man-cin");
const manCout = document.getElementById("man-cout");
const btnSubmitManual = document.getElementById("btn-submit-manual");

// Zakładka Zadania
const tasksSearch = document.getElementById("tasks-search");
const btnRefreshTasks = document.getElementById("btn-refresh-tasks");
const tasksList = document.getElementById("tasks-list");

// Pasek statusu
const statusBar = document.getElementById("status-bar");
const statusMsg = document.getElementById("status-msg");

let hasPdfStaged = false;
let hasZipStaged = false;
let allProblems = [];
let activeProblemId = null;
let isStartingDaemon = false;

// ---------------- Inicjalizacja ----------------
document.addEventListener("DOMContentLoaded", async () => {
  const urlParams = new URLSearchParams(window.location.search);
  const isTab = urlParams.get("mode") === "tab" || window.innerWidth > 550;
  if (isTab) {
    document.body.classList.add("mode-tab");
  }

  restoreStagedStorage();
  setupTabs();
  setupStaging();
  setupManualForm();
  setupTasksGallery();
  setupOfflineStarter();
  setupShutdown();
  setupDetectedHandlers();

  const autoTrigger = urlParams.get("trigger");
  if (autoTrigger === "pdf" || autoTrigger === "zip") {
    const el = autoTrigger === "pdf" ? fileInputPdf : fileInputZip;
    const card = autoTrigger === "pdf" ? dropPdf : dropZip;
    if (card) {
      card.classList.add("drop-active");
      setTimeout(() => card.classList.remove("drop-active"), 2500);
    }
    if (el) {
      let triggered = false;
      const firePicker = () => {
        if (triggered) return;
        triggered = true;
        try { el.click(); } catch (_) {}
        window.removeEventListener("pointerdown", firePicker, true);
        window.removeEventListener("click", firePicker, true);
        window.removeEventListener("keydown", firePicker, true);
        window.removeEventListener("focus", firePicker, true);
      };

      // 1. Natychmiastowe próby wywołania dialogu wyboru pliku
      firePicker();
      requestAnimationFrame(firePicker);
      setTimeout(firePicker, 50);

      // 2. Jeśli przeglądarka wymaga aktywacji użytkownika w nowej karcie:
      // Pierwszy klik w dowolnym miejscu okna lub fokus natychmiast wyzwala okno wyboru
      window.addEventListener("pointerdown", firePicker, true);
      window.addEventListener("click", firePicker, true);
      window.addEventListener("keydown", firePicker, true);
      window.addEventListener("focus", firePicker, true);
    }
  }

  // 1. Natychmiastowe ustawienie widoku (bez opóźnienia i skoków okna)
  await updatePopupViewState();

  // 2. Weryfikacja stanu daemona AlgoDeck
  const isOnline = await checkDaemonHealth();
  if (isOnline) {
    const stagedMore = await syncRecentDownloads();
    if (stagedMore) {
      await updatePopupViewState();
    }
    loadTasks();
  }

  setInterval(async () => {
    const online = await checkDaemonHealth();
    if (online) {
      const stagedMore = await syncRecentDownloads();
      if (stagedMore) {
        await updatePopupViewState();
      }
    }
  }, 3000);
});

// Obsługa widoku 1-Klik w popupie rozszerzenia
function setupDetectedHandlers() {
  const btnDetClose = document.getElementById("btn-det-close");
  const btnDetOneclick = document.getElementById("btn-det-oneclick");

  // Kliknięcie X w widoku 1-Klik:
  // Zgodnie z wymaganiem: plik POZOSTAJE w kolejce!
  // Przełączamy popup z widoku 1-Klik na widok główny (view-main) z gotową zieloną pastylką pliku!
  if (btnDetClose) {
    btnDetClose.addEventListener("click", async () => {
      let currentPdf = "";
      const detFilenameText = document.getElementById("det-filename-text");
      if (detFilenameText && detFilenameText.textContent) {
        currentPdf = detFilenameText.textContent.trim();
      }
      if (typeof chrome !== "undefined" && chrome.storage && chrome.storage.local) {
        await chrome.storage.local.set({ dismissed_pdf: currentPdf, dismiss_oneclick: true });
      }
      document.documentElement.classList.remove("detected-mode");
      document.body.classList.remove("detected-mode");
      viewDetected.classList.add("hidden");
      viewMain.classList.remove("hidden");
      await loadExistingStagedState();
    });
  }

  // Kliknięcie 1-Klik: tworzy workspace, otwiera VS Code i zamyka popup
  if (btnDetOneclick) {
    btnDetOneclick.addEventListener("click", async () => {
      btnDetOneclick.disabled = true;
      btnDetOneclick.innerHTML = `<span class="mini-spinner"></span> Tworzenie workspace...`;

      try {
        const res = await fetch(`${API_BASE}/api/start-staged`, {
          method: "POST",
          headers: { "Content-Type": "application/json" },
          body: JSON.stringify({})
        });
        const data = await res.json();
        if (res.ok && data.success) {
          if (typeof chrome !== "undefined" && chrome.action && chrome.action.setBadgeText) {
            chrome.action.setBadgeText({ text: "" });
          }
          if (typeof chrome !== "undefined" && chrome.storage && chrome.storage.local) {
            chrome.storage.local.remove(["dismissed_pdf", "dismiss_oneclick", "last_downloaded_pdf", "detected_task", "cleared_pdf_name"]);
          }

          btnDetOneclick.innerHTML = `✓ Gotowe! Uruchomiono VS Code`;
          setTimeout(() => {
            window.close();
          }, 600);
        } else {
          alert(`Błąd: ${data.error || data.detail || "Nie udało się utworzyć workspace"}`);
          btnDetOneclick.disabled = false;
          btnDetOneclick.innerHTML = `<span class="btn-bolt">⚡</span> Utwórz Workspace i Otwórz VS Code (1 Klik)`;
        }
      } catch (e) {
        alert("Błąd połączenia z AlgoDeck");
        btnDetOneclick.disabled = false;
        btnDetOneclick.innerHTML = `<span class="btn-bolt">⚡</span> Utwórz Workspace i Otwórz VS Code (1 Klik)`;
      }
    });
  }
}

// Sprawdzenie czy pokazać widok 1-Klik, czy widok główny z zakolejkowanym plikiem
async function updatePopupViewState() {
  try {
    const res = await fetch(`${API_BASE}/api/staged-state`);
    if (!res.ok) {
      await showMainView();
      return;
    }
    const state = await res.json();
    const hasPdf = Boolean(state.has_pdf || state.has_staged_pdf);

    if (!hasPdf) {
      await showMainView();
      return;
    }

    const pdfName = state.pdf_filename || (state.pdf_path ? state.pdf_path.replace(/^.*[\\\/]/, '') : "zadanie.pdf");
    const code = state.problem_id || "ZAD";

    // Sprawdź czy użytkownik zamknął krzyżykiem (X) TEN KONKRETNY plik
    let isDismissedForThisFile = false;
    if (typeof chrome !== "undefined" && chrome.storage && chrome.storage.local) {
      const stored = await chrome.storage.local.get(["dismissed_pdf"]);
      if (stored.dismissed_pdf && stored.dismissed_pdf.toLowerCase() === pdfName.toLowerCase()) {
        isDismissedForThisFile = true;
      }
    }

    // Jeśli ten konkretny plik został już odrzucony przyciskiem X, pokazujemy widok główny
    if (isDismissedForThisFile) {
      await showMainView();
      return;
    }

    // W przeciwnym razie: wyświetlamy dedykowany widok 1-Klik z wykrytym zadaniem!
    const detFilenameText = document.getElementById("det-filename-text");
    const detCodeText = document.getElementById("det-code-text");

    if (detFilenameText) detFilenameText.textContent = pdfName;
    if (detCodeText) detCodeText.textContent = code.toUpperCase();

    document.documentElement.classList.remove("offline-mode");
    document.body.classList.remove("offline-mode");
    document.documentElement.classList.add("detected-mode");
    document.body.classList.add("detected-mode");
    viewOffline.classList.add("hidden");
    viewMain.classList.add("hidden");
    viewDetected.classList.remove("hidden");
  } catch (_) {
    await showMainView();
  }
}

async function showMainView() {
  document.documentElement.classList.remove("detected-mode");
  document.body.classList.remove("detected-mode");
  document.documentElement.classList.remove("offline-mode");
  document.body.classList.remove("offline-mode");
  viewDetected.classList.add("hidden");
  viewOffline.classList.add("hidden");
  viewMain.classList.remove("hidden");
  await loadExistingStagedState();
}


// ---------------- 1. Start AlgoDeck gdy OFFLINE ----------------
function setupOfflineStarter() {
  btnStartAlgodeck.addEventListener("click", async () => {
    if (isStartingDaemon) return;
    isStartingDaemon = true;

    btnStartAlgodeck.classList.add("loading");
    btnStartAlgodeck.disabled = true;
    offlineSpinner.classList.remove("hidden");
    offlineBtnText.textContent = "Uruchamianie AlgoDeck...";

    // 1. Wyślij komunikat przez native messaging host
    let nativeErrMsg = null;
    try {
      if (typeof chrome !== "undefined" && chrome.runtime && chrome.runtime.sendNativeMessage) {
        chrome.runtime.sendNativeMessage("com.algodeck.launcher", { action: "start" }, (resp) => {
          if (chrome.runtime.lastError) {
            nativeErrMsg = chrome.runtime.lastError.message || "Błąd komunikacji native host";
            console.warn("[AlgoDeck] Native messaging fallback:", nativeErrMsg, "Ext ID:", chrome.runtime.id);
          } else {
            console.log("[AlgoDeck] Native host start response:", resp);
          }
        });
      }
    } catch (e) {
      console.warn("[AlgoDeck] Błąd native host:", e);
    }

    // 2. Odpytuj serwer co 300ms aż wstanie
    let attempts = 0;
    const maxAttempts = 40; // 12 sekund max
    const checkTimer = setInterval(async () => {
      attempts++;
      try {
        const res = await fetch(`${API_BASE}/api/status`, { cache: "no-store" });
        if (res.ok) {
          clearInterval(checkTimer);
          isStartingDaemon = false;

          // Wyświetl powiadomienie systemowe (jeśli włączone w ustawieniach)
          try {
            let allowNotify = true;
            if (typeof chrome !== "undefined" && chrome.storage && chrome.storage.local) {
              const stored = await chrome.storage.local.get(["algodeck_settings"]);
              if (stored?.algodeck_settings?.notifications === false || stored?.algodeck_settings?.notify_daemon_status === false) {
                allowNotify = false;
              }
            }
            if (allowNotify) {
              try {
                const sRes = await fetch(`${API_BASE}/api/settings`);
                if (sRes.ok) {
                  const sData = await sRes.json();
                  if (sData.notifications === false || sData.notify_daemon_status === false) allowNotify = false;
                }
              } catch (_) {}
            }
            if (allowNotify && typeof chrome !== "undefined" && chrome.notifications && chrome.notifications.create) {
              const nId = `online_${Date.now()}`;
              chrome.notifications.create(nId, {
                type: "basic",
                iconUrl: "icons/icon128.png",
                title: "AlgoDeck ⚡",
                message: "Środowisko olimpijskie zostało pomyślnie uruchomione!"
              });
              setTimeout(() => {
                try {
                  chrome.notifications.clear(nId);
                } catch (_) {}
              }, 4000);
            }
          } catch (_) {}

          // Schowaj całkowicie ekran offline i przełącz na główny widok
          viewOffline.classList.add("hidden");
          viewMain.classList.remove("hidden");
          btnStartAlgodeck.classList.remove("loading");
          btnStartAlgodeck.disabled = false;
          offlineSpinner.classList.add("hidden");
          offlineBtnText.textContent = "⚡ Uruchom AlgoDeck";

          await syncRecentDownloads();
          await updatePopupViewState();
          loadTasks();
          return;
        }
      } catch (err) {}

      if (attempts >= maxAttempts) {
        clearInterval(checkTimer);
        isStartingDaemon = false;
        btnStartAlgodeck.classList.remove("loading");
        btnStartAlgodeck.disabled = false;
        offlineSpinner.classList.add("hidden");
        offlineBtnText.textContent = "⚡ Spróbuj ponownie uruchomić";
        const extraInfo = nativeErrMsg ? `\n\nKomunikat przeglądarki: ${nativeErrMsg}\nID Twojego rozszerzenia: ${chrome?.runtime?.id || "nieznane"}` : "";
        alert(`Nie udało się połączyć z serwerem AlgoDeck.${extraInfo}\n\nMożesz też uruchomić AlgoDeck w terminalu poleceniem:\n  algodeck start`);
      }
    }, 300);
  });
}

// ---------------- 2. Sprawdzanie stanu demona ----------------
async function checkDaemonHealth() {
  if (isStartingDaemon) return false;

  try {
    const res = await fetch(`${API_BASE}/api/status`, { cache: "no-store" });
    if (res.ok) {
      // ONLINE: Schowaj widok offline
      document.documentElement.classList.remove("offline-mode");
      document.body.classList.remove("offline-mode");
      viewOffline.classList.add("hidden");

      // BARDZO WAŻNE: Jeśli widok 1-Klik jest widoczny, widok główny MUSI pozostać ukryty!
      if (!viewDetected.classList.contains("hidden")) {
        viewMain.classList.add("hidden");
      } else {
        viewMain.classList.remove("hidden");
      }
      return true;
    }
  } catch (e) {}

  // OFFLINE: Pokaż TYLKO widok offline ze zwartymi, symetrycznymi marginesami
  document.documentElement.classList.remove("detected-mode");
  document.body.classList.remove("detected-mode");
  document.documentElement.classList.add("offline-mode");
  document.body.classList.add("offline-mode");
  viewDetected.classList.add("hidden");
  viewMain.classList.add("hidden");
  viewOffline.classList.remove("hidden");
  return false;
}

// ---------------- 3. Zaciąganie świeżo pobranego zadania PDF do poczekalni ----------------
async function syncRecentDownloads() {
  try {
    let filePath = null;

    // Pobierz konfigurację ignorowania starych plików
    let installedAt = null;
    const ignoredIds = new Set();
    const ignoredFiles = new Set();

    if (typeof chrome !== "undefined" && chrome.storage && chrome.storage.local) {
      const stored = await chrome.storage.local.get([
        "last_downloaded_pdf",
        "cleared_pdf_name",
        "algodeck_installed_at",
        "initial_ignored_ids",
        "initial_ignored_files"
      ]);

      if (stored.algodeck_installed_at) {
        installedAt = stored.algodeck_installed_at;
      }
      if (Array.isArray(stored.initial_ignored_ids)) {
        stored.initial_ignored_ids.forEach(id => ignoredIds.add(id));
      }
      if (Array.isArray(stored.initial_ignored_files)) {
        stored.initial_ignored_files.forEach(f => ignoredFiles.add(f.toLowerCase()));
      }

      if (stored.last_downloaded_pdf) {
        const base = stored.last_downloaded_pdf.replace(/^.*[\\\/]/, '').toLowerCase();
        if (ignoredFiles.has(base)) {
          await chrome.storage.local.remove(["last_downloaded_pdf"]);
        } else {
          filePath = stored.last_downloaded_pdf;
        }
      }
    }

    if (!installedAt) {
      try {
        const res = await fetch(`${API_BASE}/api/initial-downloads`);
        if (res.ok) {
          const data = await res.json();
          if (data.installed_at) {
            installedAt = data.installed_at;
            if (typeof chrome !== "undefined" && chrome.storage && chrome.storage.local) {
              chrome.storage.local.set({ algodeck_installed_at: installedAt });
            }
          }
          if (Array.isArray(data.ignored_files)) {
            data.ignored_files.forEach(f => ignoredFiles.add(f.toLowerCase()));
          }
        }
      } catch (_) {}
    }

    // Fallback: przeszukaj ostatnio pobrane pliki PDF (wyłącznie pobrane PO instalacji AlgoDeck)
    if (!filePath && typeof chrome !== "undefined" && chrome.downloads && chrome.downloads.search) {
      try {
        const recent = await chrome.downloads.search({
          limit: 10,
          orderBy: ["-startTime"],
          state: "complete"
        });
        const stored = await chrome.storage.local.get(["cleared_pdf_name"]);
        const clearedName = stored.cleared_pdf_name || "";

        for (const item of recent || []) {
          if (ignoredIds.has(item.id)) continue;
          const fn = item.filename || "";
          const baseName = fn.replace(/^.*[\\\/]/, '');
          const lowerBase = baseName.toLowerCase();
          if (ignoredFiles.has(lowerBase)) continue;

          const isPdf = fn.toLowerCase().endsWith(".pdf") || (item.mime && item.mime.toLowerCase().includes("pdf"));
          if (isPdf && fn && !fn.endsWith(".crdownload") && !fn.endsWith(".tmp")) {
            if (baseName && baseName !== clearedName) {
              const startTs = item.endTime ? new Date(item.endTime).getTime() : new Date(item.startTime).getTime();
              
              // Całkowite pomijanie plików pobranych przed instalacją AlgoDeck
              if (installedAt && startTs <= installedAt) {
                continue;
              }

              const ageMs = Date.now() - startTs;
              if (ageMs < 7200000) { // 2 godziny
                filePath = fn;
                break;
              }
            }
          }
        }
      } catch (err) {
        console.warn("[AlgoDeck] chrome.downloads.search error:", err);
      }
    }

    if (filePath) {
      let success = false;
      let wasIgnored = false;

      try {
        const res = await fetch(`${API_BASE}/api/stage-by-path`, {
          method: "POST",
          headers: { "Content-Type": "application/json" },
          body: JSON.stringify({ file_path: filePath })
        });
        if (res.ok) {
          const respData = await res.json().catch(() => ({}));
          if (respData && respData.ignored) {
            wasIgnored = true;
          } else {
            success = true;
          }
        }
      } catch (_) {}

      if (wasIgnored) {
        if (typeof chrome !== "undefined" && chrome.storage && chrome.storage.local) {
          await chrome.storage.local.remove(["last_downloaded_pdf"]);
        }
        return false;
      }

      if (!success) {
        try {
          if (typeof chrome !== "undefined" && chrome.runtime && chrome.runtime.sendNativeMessage) {
            chrome.runtime.sendNativeMessage("com.algodeck.launcher", { action: "stage_file", file_path: filePath }, (resp) => {
              if (resp && resp.status === "ignored") {
                console.log("[AlgoDeck] Plik zignorowany przez native host.");
              }
            });
            success = true;
          }
        } catch (_) {}
      }

      if (success && typeof chrome !== "undefined" && chrome.storage && chrome.storage.local) {
        await chrome.storage.local.remove(["last_downloaded_pdf", "cleared_pdf_name", "dismissed_pdf", "dismiss_oneclick"]);
        return true;
      }
    }
  } catch (e) {
    console.warn("[AlgoDeck] Błąd syncRecentDownloads:", e);
  }
  return false;
}

// ---------------- 4. Nawigacja Zakładek ----------------
function setupTabs() {
  tabButtons.forEach(btn => {
    btn.addEventListener("click", () => {
      tabButtons.forEach(b => b.classList.remove("active"));
      tabPanes.forEach(p => p.classList.remove("active"));

      btn.classList.add("active");
      const targetId = btn.getAttribute("data-tab");
      const targetPane = document.getElementById(targetId);
      if (targetPane) targetPane.classList.add("active");

      if (targetId === "tab-tasks") {
        loadTasks();
      }
    });
  });

  const urlParams = new URLSearchParams(window.location.search);
  const paramTab = urlParams.get("tab");
  if (paramTab) {
    const targetBtn = document.querySelector(`.tab-btn[data-tab="tab-${paramTab}"]`) || document.querySelector(`.tab-btn[data-tab="${paramTab}"]`);
    if (targetBtn) targetBtn.click();
  }
}


// ---------------- 5. Staging Plików PDF i ZIP ----------------
function setupStaging() {
  const isLinux = /Linux/i.test(navigator.platform || navigator.userAgent);
  const isTabMode = window.location.search.includes("mode=tab") || window.innerWidth > 550;

  function triggerPicker(type) {
    if (isLinux && !isTabMode) {
      if (typeof chrome !== "undefined" && chrome.tabs && chrome.tabs.create) {
        chrome.tabs.create({ url: chrome.runtime.getURL(`popup.html?mode=tab&trigger=${type}`) });
        window.close();
        return;
      }
    }
    if (type === "pdf") fileInputPdf.click();
    else fileInputZip.click();
  }

  // Kliknięcie w dowolne miejsce karty wywołuje wybór pliku (chyba że kliknięto usunięcie)
  dropPdf.addEventListener("click", (e) => {
    if (e.target.closest("#btn-remove-pdf") || e.target.closest("#staged-pdf-pill")) return;
    triggerPicker("pdf");
  });
  fileInputPdf.addEventListener("change", (e) => {
    if (e.target.files.length > 0) uploadStagedFile(e.target.files[0], "pdf");
  });

  dropZip.addEventListener("click", (e) => {
    if (e.target.closest("#btn-remove-zip") || e.target.closest("#staged-zip-pill")) return;
    triggerPicker("zip");
  });
  fileInputZip.addEventListener("change", (e) => {
    if (e.target.files.length > 0) uploadStagedFile(e.target.files[0], "zip");
  });

  setupDropZone(dropPdf, (file) => {
    if (file.name.toLowerCase().endsWith(".pdf")) uploadStagedFile(file, "pdf");
    else showToast("Wymagany plik .pdf!", true);
  });

  setupDropZone(dropZip, (file) => {
    if (file.name.toLowerCase().endsWith(".zip")) uploadStagedFile(file, "zip");
    else showToast("Wymagany plik .zip!", true);
  });

  btnRemovePdf.addEventListener("click", async (e) => {
    e.stopPropagation();
    const removedName = stagedPdfName.textContent;
    try {
      await fetch(`${API_BASE}/api/clear-staged`, {
        method: "POST",
        headers: { "Content-Type": "application/json" },
        body: JSON.stringify({ file_type: "pdf" })
      });
    } catch (_) {}
    if (typeof chrome !== "undefined" && chrome.storage && chrome.storage.local) {
      chrome.storage.local.set({ cleared_pdf_name: removedName });
      chrome.storage.local.remove(["last_downloaded_pdf", "dismiss_oneclick", "dismissed_pdf", "detected_task"]);
    }
    if (typeof chrome !== "undefined" && chrome.action && chrome.action.setBadgeText) {
      chrome.action.setBadgeText({ text: "" });
    }
    clearStagedStorage("pdf");
    stagedPdfPill.classList.add("hidden");
    dropPdfBody.classList.remove("hidden");
    fileInputPdf.value = "";
    hasPdfStaged = false;
    updateFilesSubmitBtn();
  });

  btnRemoveZip.addEventListener("click", async (e) => {
    e.stopPropagation();
    const removedName = stagedZipName.textContent;
    try {
      await fetch(`${API_BASE}/api/clear-staged`, {
        method: "POST",
        headers: { "Content-Type": "application/json" },
        body: JSON.stringify({ file_type: "zip" })
      });
    } catch (_) {}
    if (typeof chrome !== "undefined" && chrome.storage && chrome.storage.local) {
      chrome.storage.local.set({ cleared_zip_name: removedName });
      chrome.storage.local.remove(["last_downloaded_zip"]);
    }
    clearStagedStorage("zip");
    stagedZipPill.classList.add("hidden");
    dropZipBody.classList.remove("hidden");
    fileInputZip.value = "";
    hasZipStaged = false;
    updateFilesSubmitBtn();
  });

  btnSubmitFiles.addEventListener("click", async () => {
    btnSubmitFiles.disabled = true;
    btnSubmitFiles.innerHTML = `<span class="mini-spinner"></span> Tworzenie workspace i VS Code...`;

    try {
      const res = await fetch(`${API_BASE}/api/start-staged`, {
        method: "POST",
        headers: { "Content-Type": "application/json" },
        body: JSON.stringify({})
      });
      const data = await res.json();
      if (res.ok && data.success) {
        const pid = (data.problem_id || "zad").toLowerCase();

        if (typeof chrome !== "undefined" && chrome.action && chrome.action.setBadgeText) {
          chrome.action.setBadgeText({ text: "" });
        }
        if (typeof chrome !== "undefined" && chrome.storage && chrome.storage.local) {
          chrome.storage.local.remove(["last_downloaded_pdf", "last_downloaded_zip", "cleared_pdf_name", "cleared_zip_name", "dismiss_oneclick", "dismissed_pdf", "detected_task"]);
        }
        clearStagedStorage("all");
        hasPdfStaged = false;
        hasZipStaged = false;

        showToast(`✓ Utworzono [${pid.toUpperCase()}]! Uruchomiono VS Code.`, false);
        btnSubmitFiles.innerHTML = `<span>✓ Otwarto zadanie [${pid.toUpperCase()}] w VS Code!</span>`;
        setTimeout(() => window.close(), 1200);
      } else {
        showToast(data.error || data.detail || "Błąd tworzenia workspace z plików.", true);
        btnSubmitFiles.disabled = false;
        btnSubmitFiles.innerHTML = `<span class="btn-bolt">⚡</span> Utwórz Workspace i Otwórz VS Code`;
      }
    } catch (err) {
      showToast("Błąd połączenia z backendem AlgoDeck!", true);
      btnSubmitFiles.disabled = false;
      btnSubmitFiles.innerHTML = `<span class="btn-bolt">⚡</span> Utwórz Workspace i Otwórz VS Code`;
    }
  });
}

function saveStagedStorage(pdfName, zipName) {
  if (typeof chrome !== "undefined" && chrome.storage && chrome.storage.local) {
    const data = {};
    if (pdfName !== undefined) data.algodeck_staged_pdf = pdfName || "";
    if (zipName !== undefined) data.algodeck_staged_zip = zipName || "";
    chrome.storage.local.set(data);
  }
  try {
    if (pdfName !== undefined) {
      if (pdfName) localStorage.setItem("algodeck_staged_pdf", pdfName);
      else localStorage.removeItem("algodeck_staged_pdf");
    }
    if (zipName !== undefined) {
      if (zipName) localStorage.setItem("algodeck_staged_zip", zipName);
      else localStorage.removeItem("algodeck_staged_zip");
    }
  } catch (_) {}
}

function clearStagedStorage(type = "all") {
  if (typeof chrome !== "undefined" && chrome.storage && chrome.storage.local) {
    if (type === "all") chrome.storage.local.remove(["algodeck_staged_pdf", "algodeck_staged_zip"]);
    else if (type === "pdf") chrome.storage.local.remove(["algodeck_staged_pdf"]);
    else if (type === "zip") chrome.storage.local.remove(["algodeck_staged_zip"]);
  }
  try {
    if (type === "all" || type === "pdf") localStorage.removeItem("algodeck_staged_pdf");
    if (type === "all" || type === "zip") localStorage.removeItem("algodeck_staged_zip");
  } catch (_) {}
}

function restoreStagedStorage() {
  // Stan poczekalni (staging) jest pobierany bezpośrednio i autorytatywnie
  // z backendu przez loadExistingStagedState() — nie pokazujemy przeterminowanych plików z pamięci lokalnej.
}

function setupDropZone(dropZoneEl, onFileDrop) {
  ["dragenter", "dragover"].forEach(eventName => {
    dropZoneEl.addEventListener(eventName, (e) => {
      e.preventDefault();
      e.stopPropagation();
      dropZoneEl.classList.add("drag-over");
    });
  });

  ["dragleave", "drop"].forEach(eventName => {
    dropZoneEl.addEventListener(eventName, (e) => {
      e.preventDefault();
      e.stopPropagation();
      dropZoneEl.classList.remove("drag-over");
    });
  });

  dropZoneEl.addEventListener("drop", (e) => {
    e.preventDefault();
    e.stopPropagation();
    if (e.dataTransfer.files && e.dataTransfer.files.length > 0) {
      onFileDrop(e.dataTransfer.files[0]);
    }
  });
}

async function uploadStagedFile(file, type) {
  const isPdf = (type === "pdf");
  const progressEl = isPdf ? progressPdf : progressZip;
  const bodyEl = isPdf ? dropPdfBody : dropZipBody;
  const pillEl = isPdf ? stagedPdfPill : stagedZipPill;
  const nameEl = isPdf ? stagedPdfName : stagedZipName;

  bodyEl.classList.add("hidden");
  progressEl.classList.remove("hidden");

  const formData = new FormData();
  formData.append("file", file);
  formData.append("file_type", type);

  try {
    const res = await fetch(`${API_BASE}/api/stage-file`, {
      method: "POST",
      body: formData
    });
    const data = await res.json();
    progressEl.classList.add("hidden");

    if (res.ok && (data.success || data.path)) {
      if (isPdf) {
        nameEl.textContent = file.name;
        hasPdfStaged = true;
        saveStagedStorage(file.name, undefined);
      } else {
        const testsCount = data.tests_count || 0;
        const dispName = `${file.name} (${testsCount} testów)`;
        nameEl.textContent = dispName;
        hasZipStaged = true;
        saveStagedStorage(undefined, dispName);
      }
      pillEl.classList.remove("hidden");
      showToast(`Dołączono: ${file.name}`, false);
      updateFilesSubmitBtn();
    } else {
      bodyEl.classList.remove("hidden");
      showToast(data.detail || data.error || "Błąd podczas przesyłania pliku.", true);
    }
  } catch (err) {
    progressEl.classList.add("hidden");
    bodyEl.classList.remove("hidden");
    showToast("Błąd przesyłania pliku na serwer AlgoDeck.", true);
  }
}

async function loadExistingStagedState() {
  try {
    const res = await fetch(`${API_BASE}/api/staged-state`);
    if (res.ok) {
      const state = await res.json();
      const hasPdf = state.has_pdf || state.has_staged_pdf;
      if (hasPdf) {
        hasPdfStaged = true;
        const pdfName = state.pdf_filename || (state.pdf_path ? state.pdf_path.replace(/^.*[\\\/]/, '') : "zadanie.pdf");
        stagedPdfName.textContent = pdfName;
        dropPdfBody.classList.add("hidden");
        stagedPdfPill.classList.remove("hidden");
        saveStagedStorage(pdfName, undefined);
      } else {
        hasPdfStaged = false;
        stagedPdfPill.classList.add("hidden");
        dropPdfBody.classList.remove("hidden");
        clearStagedStorage("pdf");
      }

      const hasZip = state.has_zip || state.has_staged_zip;
      if (hasZip) {
        hasZipStaged = true;
        const zipFile = state.zip_filename || (state.zip_path ? state.zip_path.replace(/^.*[\\\/]/, '') : "testy.zip");
        const zipName = `${zipFile} (${state.tests_count || 0} testów)`;
        stagedZipName.textContent = zipName;
        dropZipBody.classList.add("hidden");
        stagedZipPill.classList.remove("hidden");
        saveStagedStorage(undefined, zipName);
      } else {
        hasZipStaged = false;
        stagedZipPill.classList.add("hidden");
        dropZipBody.classList.remove("hidden");
        clearStagedStorage("zip");
      }
      updateFilesSubmitBtn();
    }
  } catch (e) {}
}

function updateFilesSubmitBtn() {
  btnSubmitFiles.disabled = !(hasPdfStaged || hasZipStaged);
}

// ---------------- 6. Ręczne Zadanie ----------------
function setupManualForm() {
  btnSubmitManual.addEventListener("click", async () => {
    const code = manCode.value.trim().toLowerCase();
    if (!code) {
      showToast("Podaj kod lub nazwę zadania!", true);
      manCode.focus();
      return;
    }

    const title = manTitle.value.trim() || code.toUpperCase();
    const timeLimit = parseFloat(manTime.value) || 1.0;
    const memoryLimit = parseInt(manMem.value, 10) || 128;
    const cinVal = manCin.value.trim();
    const coutVal = manCout.value.trim();

    const tests = [];
    if (cinVal || coutVal) {
      tests.push({
        id: "test_1",
        name: "Przykład 1",
        input: cinVal,
        expected_output: coutVal
      });
    }

    btnSubmitManual.disabled = true;
    btnSubmitManual.innerHTML = `<span class="mini-spinner"></span> Tworzenie zadania...`;

    try {
      const res = await fetch(`${API_BASE}/api/create-manual`, {
        method: "POST",
        headers: { "Content-Type": "application/json" },
        body: JSON.stringify({
          problem_id: code,
          title: title,
          time_limit: timeLimit,
          memory_limit: memoryLimit,
          sample_cin: cinVal,
          sample_cout: coutVal,
          tests: tests
        })
      });

      const data = await res.json();
      if (res.ok && data.success) {
        showToast(`✓ Utworzono zadanie [${code.toUpperCase()}]! Uruchomiono VS Code.`, false);
        btnSubmitManual.innerHTML = `<span>✓ Otwarto zadanie [${code.toUpperCase()}] w VS Code!</span>`;
        setTimeout(() => window.close(), 1200);
      } else {
        showToast(data.detail || "Błąd tworzenia zadania.", true);
        btnSubmitManual.disabled = false;
        btnSubmitManual.innerHTML = `<span class="btn-bolt">⚡</span> Utwórz Workspace i Otwórz VS Code`;
      }
    } catch (err) {
      showToast("Błąd połączenia z backendem AlgoDeck!", true);
      btnSubmitManual.disabled = false;
      btnSubmitManual.innerHTML = `<span class="btn-bolt">⚡</span> Utwórz Workspace i Otwórz VS Code`;
    }
  });
}

// ---------------- 7. Zakładka Zadania (Lista, Aktywacja, Testy ZIP, Usuwanie) ----------------
function setupTasksGallery() {
  btnRefreshTasks.addEventListener("click", loadTasks);
  tasksSearch.addEventListener("input", filterTasks);
}

async function loadTasks() {
  tasksList.innerHTML = `<div class="loading-state">Ładowanie zadań...</div>`;
  try {
    const res = await fetch(`${API_BASE}/api/problems`);
    if (res.ok) {
      const data = await res.json();
      allProblems = Array.isArray(data) ? data : (data.problems || []);
      activeProblemId = data.active || null;
      renderTasks(allProblems);
    } else {
      tasksList.innerHTML = `<div class="empty-state">Błąd pobierania listy zadań</div>`;
    }
  } catch (err) {
    tasksList.innerHTML = `<div class="empty-state">Brak połączenia z serwerem</div>`;
  }
}

function filterTasks() {
  const query = tasksSearch.value.toLowerCase().trim();
  if (!query) {
    renderTasks(allProblems);
    return;
  }
  const filtered = allProblems.filter(p => {
    const pid = (p.problem_id || "").toLowerCase();
    const ptitle = (p.title || "").toLowerCase();
    return pid.includes(query) || ptitle.includes(query);
  });
  renderTasks(filtered);
}

function renderTasks(problems) {
  if (!problems || problems.length === 0) {
    tasksList.innerHTML = `<div class="empty-state">Brak zapisanych zadań</div>`;
    return;
  }

  tasksList.innerHTML = "";
  problems.forEach(p => {
    const pid = p.problem_id || "zad";
    const title = p.title || pid.toUpperCase();
    const testsCount = p.tests_count !== undefined ? p.tests_count : (p.tests || []).length;
    const isActive = (pid.toLowerCase() === (activeProblemId || "").toLowerCase());

    const card = document.createElement("div");
    card.className = `task-card ${isActive ? "active-task-card" : ""}`;
    card.innerHTML = `
      <div class="task-card-header" title="Kliknij, aby otworzyć zadanie w VS Code i aktywować na Stream Decku">
        <span class="task-code-badge">${escapeHtml(pid.toUpperCase())}</span>
        <span class="task-title" title="${escapeHtml(title)}">${escapeHtml(title)}</span>
        <span class="task-meta">${testsCount} testów</span>
      </div>
      <div class="task-actions">
        <button type="button" class="btn-task-action btn-task-activate" title="Przełącz Stream Deck i otwórz w VS Code">Aktywuj</button>
        <button type="button" class="btn-task-action btn-task-add-tests" title="Wgraj paczkę testów .zip">+ Testy (ZIP)</button>
        <button type="button" class="btn-task-action btn-task-manual-toggle" title="Wpisz wejście i wyjście nowego testu">+ Testy</button>
        <button type="button" class="btn-task-action btn-task-delete" title="Usuń zadanie">Usuń</button>
      </div>
      <div class="manual-test-drawer hidden">
        <div class="manual-test-header">
          <span class="manual-test-title">➕ Dodaj test ręczny dla zadania '${escapeHtml(pid.toUpperCase())}'</span>
        </div>
        <div class="manual-test-grid">
          <div class="manual-test-col">
            <label class="manual-test-label">Wejście testu (cin / input):</label>
            <textarea class="manual-test-textarea manual-in" rows="3" placeholder="Wklej wejście testu..." spellcheck="false"></textarea>
          </div>
          <div class="manual-test-col">
            <label class="manual-test-label">Oczekiwane wyjście (cout / output):</label>
            <textarea class="manual-test-textarea manual-out" rows="3" placeholder="Wklej oczekiwane wyjście..." spellcheck="false"></textarea>
          </div>
        </div>
        <div class="manual-test-footer">
          <button type="button" class="btn-save-manual-test btn-primary-compact">Zapisz test</button>
          <button type="button" class="btn-cancel-manual-test btn-secondary-compact">Anuluj</button>
        </div>
      </div>
    `;

    // Kliknięcie w nagłówek (nazwę zadania) otwiera w VS Code i aktywuje zadanie
    card.querySelector(".task-card-header").addEventListener("click", async () => {
      try {
        await fetch(`${API_BASE}/api/set-active/${pid}`, { method: "POST" });
        await fetch(`${API_BASE}/api/open-vscode/${pid}`, { method: "POST" });
        activeProblemId = pid;
        showToast(`Otwarto '${pid.toUpperCase()}' w VS Code!`, false);
        loadTasks();
      } catch (e) {
        showToast("Błąd otwierania zadania w VS Code", true);
      }
    });

    // Aktywuj zadanie (Stream Deck + VS Code)
    card.querySelector(".btn-task-activate").addEventListener("click", async () => {
      try {
        await fetch(`${API_BASE}/api/set-active/${pid}`, { method: "POST" });
        await fetch(`${API_BASE}/api/open-vscode/${pid}`, { method: "POST" });
        activeProblemId = pid;
        showToast(`Aktywowano '${pid.toUpperCase()}' na Stream Decku i w VS Code!`, false);
        loadTasks();
      } catch (e) {
        showToast("Błąd aktywacji zadania", true);
      }
    });

    // Szuflada dodawania testów ręcznie
    const drawer = card.querySelector(".manual-test-drawer");
    const toggleBtn = card.querySelector(".btn-task-manual-toggle");
    const inputEl = card.querySelector(".manual-in");
    const outputEl = card.querySelector(".manual-out");
    const btnSave = card.querySelector(".btn-save-manual-test");
    const btnCancel = card.querySelector(".btn-cancel-manual-test");

    toggleBtn.addEventListener("click", () => {
      const isClosed = drawer.classList.contains("hidden");
      if (isClosed) {
        drawer.classList.remove("hidden");
        toggleBtn.classList.add("active");
        inputEl.focus();
      } else {
        drawer.classList.add("hidden");
        toggleBtn.classList.remove("active");
      }
    });

    btnCancel.addEventListener("click", () => {
      drawer.classList.add("hidden");
      toggleBtn.classList.remove("active");
      inputEl.value = "";
      outputEl.value = "";
    });

    btnSave.addEventListener("click", async () => {
      const inVal = inputEl.value;
      const outVal = outputEl.value;
      if (!inVal.trim() && !outVal.trim()) {
        showToast("Wpisz wejście lub wyjście testu!", true);
        return;
      }
      btnSave.disabled = true;
      btnSave.textContent = "Zapisywanie...";
      try {
        const res = await fetch(`${API_BASE}/api/problems/${pid}/add-manual-test`, {
          method: "POST",
          headers: { "Content-Type": "application/json" },
          body: JSON.stringify({ input: inVal, output: outVal })
        });
        const data = await res.json();
        if (res.ok && data.success) {
          showToast(`✓ Dodano test ręczny (łącznie: ${data.total_tests})!`, false);
          drawer.classList.add("hidden");
          toggleBtn.classList.remove("active");
          inputEl.value = "";
          outputEl.value = "";
          loadTasks();
        } else {
          showToast(data.detail || data.error || "Błąd zapisu testu!", true);
        }
      } catch (err) {
        showToast("Błąd połączenia z serwerem!", true);
      } finally {
        btnSave.disabled = false;
        btnSave.textContent = "Zapisz test";
      }
    });

    // Dodaj testy z pliku ZIP
    card.querySelector(".btn-task-add-tests").addEventListener("click", () => {
      const fileInput = document.createElement("input");
      fileInput.type = "file";
      fileInput.accept = ".zip";
      fileInput.onchange = async () => {
        if (!fileInput.files || fileInput.files.length === 0) return;
        const file = fileInput.files[0];
        const formData = new FormData();
        formData.append("file", file);
        showToast(`Wgrywanie testów dla '${pid.toUpperCase()}'...`, false);
        try {
          const res = await fetch(`${API_BASE}/api/problems/${pid}/add-tests`, {
            method: "POST",
            body: formData
          });
          const data = await res.json();
          if (res.ok && data.success) {
            showToast(`✓ Dodano ${data.tests_added} testów (łącznie: ${data.total_tests})!`, false);
            loadTasks();
          } else {
            showToast(data.detail || "Błąd dodawania testów!", true);
          }
        } catch (err) {
          showToast("Błąd połączenia z serwerem!", true);
        }
      };
      fileInput.click();
    });

    // Usuń zadanie
    card.querySelector(".btn-task-delete").addEventListener("click", async () => {
      if (!confirm(`Czy na pewno usunąć zadanie '${pid.toUpperCase()}' z workspace?`)) {
        return;
      }
      try {
        const res = await fetch(`${API_BASE}/api/problem/${pid}`, { method: "DELETE" });
        if (res.ok) {
          showToast(`Usunięto zadanie '${pid.toUpperCase()}'`, false);
          loadTasks();
        } else {
          showToast("Błąd podczas usuwania zadania!", true);
        }
      } catch (err) {
        showToast("Błąd połączenia z serwerem!", true);
      }
    });

    tasksList.appendChild(card);
  });
}

function showToast(msg, isError = false) {
  statusMsg.textContent = msg;
  statusBar.className = `status-bar ${isError ? 'error' : 'success'}`;
  statusBar.classList.remove("hidden");
  setTimeout(() => {
    statusBar.classList.add("hidden");
  }, 3500);
}

function escapeHtml(str) {
  if (!str) return "";
  return str.replace(/&/g, "&amp;").replace(/</g, "&lt;").replace(/>/g, "&gt;").replace(/"/g, "&quot;");
}

// ---------------- 8. Zatrzymywanie AlgoDeck (Wyłącz) ----------------
function setupShutdown() {
  const btnStop = document.getElementById("btn-header-stop");
  if (!btnStop) return;

  btnStop.addEventListener("click", async () => {
    btnStop.disabled = true;
    btnStop.textContent = "Zatrzymywanie...";

    // 1. Wyślij stop przez Native Messaging Host
    try {
      if (typeof chrome !== "undefined" && chrome.runtime && chrome.runtime.sendNativeMessage) {
        chrome.runtime.sendNativeMessage("com.algodeck.launcher", { action: "stop" }, () => {});
      }
    } catch (_) {}

    // 2. Wyślij stop przez HTTP API
    try {
      await fetch(`${API_BASE}/api/shutdown`, { method: "POST" });
    } catch (_) {}

    // 3. Przełącz na ekran offline
    document.documentElement.classList.add("offline-mode");
    document.body.classList.add("offline-mode");
    viewMain.classList.add("hidden");
    viewOffline.classList.remove("hidden");
    btnStop.disabled = false;
    btnStop.textContent = "Wyłącz";
  });

  function openOrFocusBrowserTab(urlKeyword, fallbackUrl, targetWindowName) {
    if (typeof chrome !== "undefined" && chrome.tabs && chrome.tabs.query) {
      chrome.tabs.query({}, (tabs) => {
        const match = (tabs || []).find(t => t.url && t.url.includes(urlKeyword));
        if (match) {
          chrome.tabs.update(match.id, { active: true });
          if (chrome.windows && match.windowId) {
            chrome.windows.update(match.windowId, { focused: true });
          }
        } else {
          const fullUrl = (chrome.runtime && chrome.runtime.getURL) ? chrome.runtime.getURL(urlKeyword) : fallbackUrl;
          chrome.tabs.create({ url: fullUrl });
        }
      });
    } else {
      window.open(fallbackUrl, targetWindowName || "_blank");
    }
  }

  const btnSettings = document.getElementById("btn-header-settings");
  if (btnSettings) {
    btnSettings.addEventListener("click", () => {
      openOrFocusBrowserTab("settings.html", "http://127.0.0.1:8080/settings", "algodeck_settings");
    });
  }

  const btnHeaderTab = document.getElementById("btn-header-tab");
  if (btnHeaderTab) {
    if (window.location.search.includes("mode=tab") || window.innerWidth > 550) {
      btnHeaderTab.style.display = "none";
    } else {
      btnHeaderTab.addEventListener("click", () => {
        openOrFocusBrowserTab("popup.html?mode=tab", "http://127.0.0.1:8080/popup.html?mode=tab", "algodeck_tab");
        window.close();
      });
    }
  }

  const btnLinkTab = document.getElementById("btn-link-tab");
  if (btnLinkTab) {
    btnLinkTab.addEventListener("click", () => {
      openOrFocusBrowserTab("popup.html?mode=tab", "http://127.0.0.1:8080/popup.html?mode=tab", "algodeck_tab");
      window.close();
    });
  }

  // Live WebSocket dla popupa / pełnej karty — nawigacja i synchronizacja
  try {
    const wsProto = window.location.protocol === "https:" ? "wss:" : "ws:";
    const wsHost = window.location.host || "127.0.0.1:8080";
    const ws = new WebSocket(`${wsProto}//${wsHost}/ws`);
    ws.onmessage = (e) => {
      if (typeof e.data === "string") {
        if (e.data === "navigate:new_task") {
          switchTab("tab-files");
          window.focus();
        } else if (e.data.startsWith("active:")) {
          const pid = e.data.substring(7);
          activeProblemId = pid || null;
          loadTasks();
        }
      }
    };
  } catch (_) {}
}

