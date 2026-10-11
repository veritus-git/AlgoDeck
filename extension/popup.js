/* ==========================================================================
   AlgoDeck Extension - Client Logic (Koło MAP)
   Widok Uruchamiania "URUCHOM" + Izolowany Widok Wykrytego Zadania
   Obsługa Native Messaging, Drag-and-Drop, Staging PDF/ZIP oraz Stanów Online/Offline
   ========================================================================== */

const API_BASE = "http://127.0.0.1:8080";

// Globalny stan
let activeProblemId = null;
let allProblems = [];
let isDaemonOnline = false;

// Elementy Widoku Launcher
const viewLauncher = document.getElementById("view-launcher");
const viewDetected = document.getElementById("view-detected");
const btnRoundLaunch = document.getElementById("btn-round-launch");
const launchHint = document.getElementById("launch-hint");
const launcherActiveBar = document.getElementById("launcher-active-bar");
const launcherActiveName = document.getElementById("launcher-active-name");
const daemonStatus = document.getElementById("daemon-status");
const statusText = document.getElementById("status-text");

// Elementy Szybkich Akcji
const btnQuickSettings = document.getElementById("btn-quick-settings");
const btnQuickIdle = document.getElementById("btn-quick-idle");
const btnToggleWorkspace = document.getElementById("btn-toggle-workspace");
const workspaceDrawer = document.getElementById("workspace-drawer");

// Staging: PDF i ZIP
const dropPdf = document.getElementById("drop-pdf");
const dropZip = document.getElementById("drop-zip");
const btnBrowsePdf = document.getElementById("btn-browse-pdf");
const btnBrowseZip = document.getElementById("btn-browse-zip");
const fileInputPdf = document.getElementById("file-input-pdf");
const fileInputZip = document.getElementById("file-input-zip");
const stagedPdfCard = document.getElementById("staged-pdf-card");
const stagedZipCard = document.getElementById("staged-zip-card");
const stagedPdfName = document.getElementById("staged-pdf-name");
const stagedZipName = document.getElementById("staged-zip-name");
const btnRemovePdf = document.getElementById("btn-remove-pdf");
const btnRemoveZip = document.getElementById("btn-remove-zip");
const btnStartStaged = document.getElementById("btn-start-staged");

// Elementy Zakładek Workspace (Drawer)
const tabButtons = document.querySelectorAll(".tab-btn");
const tabPanes = document.querySelectorAll(".tab-pane");
const btnCreateManual = document.getElementById("btn-create-manual");
const manCode = document.getElementById("man-code");
const manTitle = document.getElementById("man-title");
const gallerySearch = document.getElementById("gallery-search");
const btnRefreshGallery = document.getElementById("btn-refresh-gallery");
const galleryList = document.getElementById("gallery-list");

// ---------------- Inicjalizacja ----------------
document.addEventListener("DOMContentLoaded", async () => {
  setupTabs();
  setupLauncher();
  setupWorkspaceDrawer();
  setupStaging();

  // Sprawdź czy otworzono w trybie wykrytego zadania (Powiadomienie 1-Klik)
  const isDetected = await checkDetectedMode();

  if (!isDetected) {
    document.body.classList.remove("detected-mode");
    viewDetected.classList.add("hidden");
    viewLauncher.classList.remove("hidden");
    
    // Domyślnie startujemy w trybie offline dopóki serwer nie odpowie
    setLauncherOffline();
    await checkDaemonHealth();
    setInterval(checkDaemonHealth, 2500);
  }
});

// ---------------- 1. Sprawdzanie i obsługa trybu Wykrytego Zadania (1-Klik) ----------------
async function checkDetectedMode() {
  const urlParams = new URLSearchParams(window.location.search);
  let autoPdf = urlParams.get("auto_pdf");

  if (!autoPdf && typeof chrome !== "undefined" && chrome.storage && chrome.storage.local) {
    try {
      const stored = await chrome.storage.local.get(["auto_pdf", "auto_pdf_ts"]);
      if (stored.auto_pdf && stored.auto_pdf_ts && (Date.now() - stored.auto_pdf_ts < 60000)) {
        autoPdf = stored.auto_pdf;
        chrome.storage.local.remove(["auto_pdf", "auto_pdf_ts"]);
      }
    } catch (e) {}
  }

  if (!autoPdf) {
    return false;
  }

  document.body.classList.add("detected-mode");
  viewLauncher.classList.add("hidden");
  viewDetected.classList.remove("hidden");

  const baseName = autoPdf.replace(/^.*[\\\/]/, '');
  const cleanStem = baseName.replace(/\.[^/.]+$/, "").replace(/[^a-zA-Z0-9]/g, "").toLowerCase().slice(0, 5);
  let problemCode = cleanStem.toUpperCase() || "ZAD";

  const detFilenameText = document.getElementById("det-filename-text");
  const detCodeText = document.getElementById("det-code-text");
  const btnDetClose = document.getElementById("btn-det-close");
  const btnDetOneclick = document.getElementById("btn-det-oneclick");

  detFilenameText.textContent = baseName;
  detCodeText.textContent = problemCode;

  try {
    const res = await fetch(`${API_BASE}/api/auto-import-pdf`, {
      method: "POST",
      headers: { "Content-Type": "application/json" },
      body: JSON.stringify({ file_path: autoPdf, auto_create: false })
    });
    if (res.ok) {
      const data = await res.json();
      if (data.problem_id) {
        problemCode = data.problem_id.toUpperCase();
        detCodeText.textContent = problemCode;
      }
    }
  } catch (e) {}

  btnDetOneclick.onclick = async () => {
    btnDetOneclick.disabled = true;
    btnDetOneclick.innerHTML = `<span class="btn-bolt">⚡</span> Tworzenie workspace i VS Code...`;

    try {
      const res = await fetch(`${API_BASE}/api/auto-import-pdf`, {
        method: "POST",
        headers: { "Content-Type": "application/json" },
        body: JSON.stringify({
          file_path: autoPdf,
          problem_id: problemCode.toLowerCase(),
          auto_create: true
        })
      });
      const data = await res.json();
      if (res.ok && data.success) {
        btnDetOneclick.innerHTML = `<span>✓ Utworzono [${problemCode}]!</span>`;
        if (typeof chrome !== "undefined" && chrome.storage && chrome.storage.local) {
          chrome.storage.local.remove(["auto_pdf", "auto_pdf_ts", "auto_is_zip"]);
        }
        setTimeout(() => window.close(), 600);
      } else {
        alert(data.detail || "Błąd podczas tworzenia workspace");
        btnDetOneclick.disabled = false;
        btnDetOneclick.innerHTML = `<span class="btn-bolt">⚡</span> Utwórz Workspace i Otwórz VS Code (1 Klik)`;
      }
    } catch (err) {
      alert("Błąd połączenia z backendem AlgoDeck!");
      btnDetOneclick.disabled = false;
      btnDetOneclick.innerHTML = `<span class="btn-bolt">⚡</span> Utwórz Workspace i Otwórz VS Code (1 Klik)`;
    }
  };

  btnDetClose.onclick = () => {
    if (typeof chrome !== "undefined" && chrome.storage && chrome.storage.local) {
      chrome.storage.local.remove(["auto_pdf", "auto_pdf_ts", "auto_is_zip"]);
    }
    window.close();
  };

  return true;
}

// ---------------- 2. Zielony Okrągły Przycisk "URUCHOM" + Native Messaging ----------------
function setupLauncher() {
  btnRoundLaunch.addEventListener("click", async () => {
    btnRoundLaunch.classList.add("running");
    const launchTitle = btnRoundLaunch.querySelector(".launch-title");
    if (launchTitle) launchTitle.textContent = "START...";
    launchHint.textContent = "Uruchamianie środowiska AlgoDeck...";

    // 1. Wywołaj natywny host (działa zawsze, nawet gdy daemon i StreamController są całkowicie wyłączone!)
    if (typeof chrome !== "undefined" && chrome.runtime && chrome.runtime.sendNativeMessage) {
      try {
        chrome.runtime.sendNativeMessage(
          "com.algodeck.launcher",
          { action: "start" },
          (resp) => {
            console.log("AlgoDeck native launcher response:", resp, chrome.runtime.lastError);
          }
        );
      } catch (e) {
        console.error("Native message trigger error:", e);
      }
    }

    // 2. Wyślij zapytanie HTTP na wypadek gdyby daemon już startował
    fetch(`${API_BASE}/api/launch`, { method: "POST" }).catch(() => {});

    // 3. Odpytuj status co 350ms, aż serwer odpowie
    let attempts = 0;
    const pollInterval = setInterval(async () => {
      attempts++;
      try {
        const res = await fetch(`${API_BASE}/api/status`, { cache: "no-store" });
        if (res.ok) {
          clearInterval(pollInterval);
          const data = await res.json();
          setLauncherOnline(data);
          showToast("AlgoDeck uruchomiony! ⚡", "success");
        }
      } catch (e) {}

      if (attempts > 35) {
        clearInterval(pollInterval);
        if (!isDaemonOnline) {
          launchHint.textContent = "Uruchom w terminalu wpisując: algodeck";
          btnRoundLaunch.classList.remove("running");
          if (launchTitle) launchTitle.textContent = "URUCHOM";
        }
      }
    }, 350);
  });

  btnQuickIdle.addEventListener("click", async () => {
    try {
      await fetch(`${API_BASE}/api/streamdeck/idle`, { method: "POST" });
      showToast("Włączono ekran zachęty (ALGO_IDLE)", "success");
    } catch (e) {
      showToast("Serwer AlgoDeck jest offline", "error");
    }
  });

  btnQuickSettings.addEventListener("click", async () => {
    try {
      await fetch(`${API_BASE}/api/settings/open-gui`, { method: "POST" });
    } catch (e) {
      showToast("Otwórz terminal i wpisz: algodeck settings", "info");
    }
  });

  btnToggleWorkspace.addEventListener("click", () => {
    workspaceDrawer.classList.toggle("hidden");
    const isHidden = workspaceDrawer.classList.contains("hidden");
    btnToggleWorkspace.textContent = isHidden ? "📋 Zadania ▾" : "📋 Zwiń ▴";
    if (!isHidden) {
      fetchProblemsList();
      fetchStagedState();
    }
  });
}

// ---------------- 3. Zarządzanie Stanami Offline vs Online (Req 3) ----------------
function setLauncherOffline() {
  isDaemonOnline = false;
  viewLauncher.classList.remove("is-online");
  viewLauncher.classList.add("is-offline");

  daemonStatus.className = "status-pill status-offline";
  statusText.textContent = "OFFLINE";
  launchHint.textContent = "Kliknij, aby aktywować Stream Deck i środowisko";

  btnRoundLaunch.classList.remove("running");
  const launchTitle = btnRoundLaunch.querySelector(".launch-title");
  if (launchTitle) launchTitle.textContent = "URUCHOM";
}

function setLauncherOnline(data) {
  isDaemonOnline = true;
  viewLauncher.classList.remove("is-offline");
  viewLauncher.classList.add("is-online");

  daemonStatus.className = "status-pill status-online";
  statusText.textContent = "ONLINE";

  const activeTask = data && data.active_problem ? data.active_problem.toUpperCase() : "ALGO_IDLE";
  launcherActiveBar.classList.remove("hidden");
  launcherActiveName.textContent = activeTask;
  activeProblemId = activeTask;

  // Odśwież stan poczekalni i zadań
  fetchStagedState();
  fetchProblemsList();
}

async function checkDaemonHealth() {
  try {
    const res = await fetch(`${API_BASE}/api/status`, { cache: "no-store" });
    if (res.ok) {
      const data = await res.json();
      setLauncherOnline(data);
    } else {
      setLauncherOffline();
    }
  } catch (err) {
    setLauncherOffline();
  }
}

// ---------------- 4. Obsługa Staging (Przycisk "Wybierz", Drag&Drop, PDF/ZIP - Req 8) ----------------
function setupStaging() {
  // Przycisk "Wybierz" dla PDF
  if (btnBrowsePdf && fileInputPdf) {
    btnBrowsePdf.addEventListener("click", (e) => {
      e.stopPropagation();
      fileInputPdf.click();
    });
    fileInputPdf.addEventListener("change", (e) => {
      if (e.target.files && e.target.files[0]) {
        uploadStagedFile(e.target.files[0], "pdf");
      }
    });
  }

  // Przycisk "Wybierz" dla ZIP
  if (btnBrowseZip && fileInputZip) {
    btnBrowseZip.addEventListener("click", (e) => {
      e.stopPropagation();
      fileInputZip.click();
    });
    fileInputZip.addEventListener("change", (e) => {
      if (e.target.files && e.target.files[0]) {
        uploadStagedFile(e.target.files[0], "zip");
      }
    });
  }

  // Drag & drop na drop-pdf
  if (dropPdf) {
    setupDropzone(dropPdf, "pdf");
  }

  // Drag & drop na drop-zip
  if (dropZip) {
    setupDropzone(dropZip, "zip");
  }

  // Usuwanie pliku PDF z poczekalni
  if (btnRemovePdf) {
    btnRemovePdf.addEventListener("click", async (e) => {
      e.stopPropagation();
      try {
        await fetch(`${API_BASE}/api/clear-staged`, {
          method: "POST",
          headers: { "Content-Type": "application/json" },
          body: JSON.stringify({ file_type: "pdf" })
        });
        fileInputPdf.value = "";
        fetchStagedState();
      } catch (err) {
        showToast("Błąd usuwania pliku", "error");
      }
    });
  }

  // Usuwanie pliku ZIP z poczekalni
  if (btnRemoveZip) {
    btnRemoveZip.addEventListener("click", async (e) => {
      e.stopPropagation();
      try {
        await fetch(`${API_BASE}/api/clear-staged`, {
          method: "POST",
          headers: { "Content-Type": "application/json" },
          body: JSON.stringify({ file_type: "zip" })
        });
        fileInputZip.value = "";
        fetchStagedState();
      } catch (err) {
        showToast("Błąd usuwania pliku", "error");
      }
    });
  }

  // Start importu z plików w poczekalni
  if (btnStartStaged) {
    btnStartStaged.addEventListener("click", async () => {
      btnStartStaged.disabled = true;
      btnStartStaged.innerHTML = `<span>⚡ Tworzenie workspace...</span>`;
      try {
        const res = await fetch(`${API_BASE}/api/start-staged`, {
          method: "POST",
          headers: { "Content-Type": "application/json" },
          body: JSON.stringify({})
        });
        if (res.ok) {
          showToast("Utworzono workspace i uruchomiono VS Code! ⚡", "success");
          fetchStagedState();
          fetchProblemsList();
          checkDaemonHealth();
        } else {
          const err = await res.json();
          showToast(err.detail || "Błąd podczas importu", "error");
        }
      } catch (e) {
        showToast("Błąd połączenia z serwerem AlgoDeck", "error");
      } finally {
        btnStartStaged.disabled = false;
        btnStartStaged.innerHTML = `<span>⚡ Rozpocznij import</span>`;
      }
    });
  }
}

function setupDropzone(el, type) {
  ["dragenter", "dragover"].forEach(evt => {
    el.addEventListener(evt, (e) => {
      e.preventDefault();
      e.stopPropagation();
      el.classList.add("dragover");
    });
  });

  ["dragleave", "dragend"].forEach(evt => {
    el.addEventListener(evt, (e) => {
      e.preventDefault();
      e.stopPropagation();
      el.classList.remove("dragover");
    });
  });

  el.addEventListener("drop", (e) => {
    e.preventDefault();
    e.stopPropagation();
    el.classList.remove("dragover");

    if (e.dataTransfer && e.dataTransfer.files && e.dataTransfer.files[0]) {
      const file = e.dataTransfer.files[0];
      uploadStagedFile(file, type);
    }
  });
}

async function uploadStagedFile(file, type) {
  showToast(`Wgrywanie ${file.name}...`, "info");
  const formData = new FormData();
  formData.append("file", file);
  formData.append("file_type", type);

  try {
    const res = await fetch(`${API_BASE}/api/stage-file`, {
      method: "POST",
      body: formData
    });
    if (res.ok) {
      showToast(`Wgrano ${file.name}!`, "success");
      fetchStagedState();
    } else {
      const err = await res.json();
      showToast(err.detail || "Błąd wgrywania", "error");
    }
  } catch (e) {
    showToast("Błąd połączenia z serwerem", "error");
  }
}

async function fetchStagedState() {
  if (!isDaemonOnline) return;
  try {
    const res = await fetch(`${API_BASE}/api/staged-state`);
    if (res.ok) {
      const data = await res.json();
      updateStagingUI(data);
    }
  } catch (e) {}
}

function updateStagingUI(state) {
  // Stan PDF
  if (state.has_pdf) {
    dropPdf.classList.add("hidden");
    stagedPdfCard.classList.remove("hidden");
    stagedPdfName.textContent = state.pdf_filename || "zadanie.pdf";
  } else {
    dropPdf.classList.remove("hidden");
    stagedPdfCard.classList.add("hidden");
  }

  // Stan ZIP
  if (state.has_zip) {
    dropZip.classList.add("hidden");
    stagedZipCard.classList.remove("hidden");
    stagedZipName.textContent = state.zip_filename || "testy.zip";
  } else {
    dropZip.classList.remove("hidden");
    stagedZipCard.classList.add("hidden");
  }

  // Przycisk startu importu
  if (state.has_pdf || state.has_zip) {
    btnStartStaged.classList.remove("hidden");
  } else {
    btnStartStaged.classList.add("hidden");
  }
}

// ---------------- 5. Zakładki Drawer Workspace ----------------
function setupTabs() {
  tabButtons.forEach(btn => {
    btn.addEventListener("click", () => {
      const tabId = btn.dataset.tab;
      tabButtons.forEach(b => b.classList.remove("active"));
      tabPanes.forEach(p => p.classList.remove("active"));
      btn.classList.add("active");
      const target = document.getElementById(tabId);
      if (target) target.classList.add("active");
      if (tabId === "tab-gallery") fetchProblemsList();
    });
  });
}

function setupWorkspaceDrawer() {
  btnCreateManual.addEventListener("click", async () => {
    const code = manCode.value.trim().toLowerCase();
    if (!code) {
      showToast("Podaj kod zadania!", "error");
      return;
    }
    const title = manTitle.value.trim() || code.toUpperCase();
    btnCreateManual.disabled = true;
    try {
      const res = await fetch(`${API_BASE}/api/create-manual`, {
        method: "POST",
        headers: { "Content-Type": "application/json" },
        body: JSON.stringify({
          problem_id: code,
          title: title,
          time_limit: 1.0,
          memory_limit: 128,
          tests: []
        })
      });
      if (res.ok) {
        showToast(`Utworzono ${code.toUpperCase()}! Otwarto VS Code.`, "success");
        manCode.value = "";
        manTitle.value = "";
        fetchProblemsList();
        checkDaemonHealth();
      }
    } catch (e) {
      showToast("Błąd tworzenia zadania", "error");
    } finally {
      btnCreateManual.disabled = false;
    }
  });

  if (btnRefreshGallery) {
    btnRefreshGallery.addEventListener("click", fetchProblemsList);
  }
  if (gallerySearch) {
    gallerySearch.addEventListener("input", filterGalleryList);
  }
}

async function fetchProblemsList() {
  if (!galleryList || !isDaemonOnline) return;
  galleryList.innerHTML = `<div class="loading-state">Ładowanie zadań...</div>`;
  try {
    const res = await fetch(`${API_BASE}/api/problems`);
    if (res.ok) {
      const data = await res.json();
      allProblems = data.problems || [];
      activeProblemId = data.active || "";
      renderGallery();
    }
  } catch (e) {
    galleryList.innerHTML = `<div class="loading-state">Brak połączenia z daemonem</div>`;
  }
}

function renderGallery() {
  if (!galleryList) return;
  if (allProblems.length === 0) {
    galleryList.innerHTML = `<div class="loading-state">Brak zadań w workspace</div>`;
    return;
  }
  galleryList.innerHTML = "";
  allProblems.forEach(p => {
    const card = document.createElement("div");
    card.className = "gallery-card" + (p.problem_id === activeProblemId ? " active" : "");
    card.innerHTML = `
      <div style="font-weight: 700; color: #38bdf8;">${escapeHtml(p.title)} [${escapeHtml(p.problem_id.toUpperCase())}]</div>
      <div style="font-size: 11px; color: #94a3b8; margin-top: 4px;">Testów: ${p.tests ? p.tests.length : 0}</div>
    `;
    card.onclick = () => {
      fetch(`${API_BASE}/api/set-active/${p.problem_id}`, { method: "POST" })
        .then(() => fetchProblemsList());
    };
    galleryList.appendChild(card);
  });
}

function filterGalleryList() {
  const q = (gallerySearch.value || "").toLowerCase();
  const filtered = allProblems.filter(p => p.problem_id.includes(q) || p.title.toLowerCase().includes(q));
  galleryList.innerHTML = "";
  filtered.forEach(p => {
    const card = document.createElement("div");
    card.className = "gallery-card" + (p.problem_id === activeProblemId ? " active" : "");
    card.innerHTML = `<div style="font-weight:700; color:#38bdf8;">${escapeHtml(p.title)} [${escapeHtml(p.problem_id.toUpperCase())}]</div>`;
    card.onclick = () => {
      fetch(`${API_BASE}/api/set-active/${p.problem_id}`, { method: "POST" }).then(() => fetchProblemsList());
    };
    galleryList.appendChild(card);
  });
}

// ---------------- Toast i narzędzia ----------------
function showToast(msg, type = "info") {
  const container = document.getElementById("toast-container");
  if (!container) return;
  const toast = document.createElement("div");
  toast.className = `toast toast-${type}`;
  toast.textContent = msg;
  container.appendChild(toast);
  setTimeout(() => toast.remove(), 3500);
}

function escapeHtml(str) {
  return (str || "").replace(/[&<>"']/g, m => ({
    "&": "&amp;", "<": "&lt;", ">": "&gt;", '"': "&quot;", "'": "&#39;"
  })[m]);
}
