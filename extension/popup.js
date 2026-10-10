/* ==========================================================================
   AlgoDeck Extension - Client Logic (Koło MAP)
   ========================================================================== */

const API_BASE = "http://127.0.0.1:8080";

// Globalny stan
let activeProblemId = null;
let allProblems = [];

// DOM Elements: Status & Header
const daemonStatus = document.getElementById("daemon-status");
const statusText = document.getElementById("status-text");
const activeTaskBar = document.getElementById("active-task-bar");
const activeTaskName = document.getElementById("active-task-name");
const btnQuickVscode = document.getElementById("btn-quick-vscode");

// Zakładki
const tabButtons = document.querySelectorAll(".tab-btn");
const tabPanes = document.querySelectorAll(".tab-pane");

// Tab 1: Staging PDF
const dropPdf = document.getElementById("drop-pdf");
const btnBrowsePdf = document.getElementById("btn-browse-pdf");
const fileInputPdf = document.getElementById("file-input-pdf");
const stagedPdfCard = document.getElementById("staged-pdf-card");
const stagedPdfName = document.getElementById("staged-pdf-name");
const stagedPdfMeta = document.getElementById("staged-pdf-meta");
const btnRemovePdf = document.getElementById("btn-remove-pdf");

// Tab 1: Staging ZIP
const dropZip = document.getElementById("drop-zip");
const btnBrowseZip = document.getElementById("btn-browse-zip");
const fileInputZip = document.getElementById("file-input-zip");
const stagedZipCard = document.getElementById("staged-zip-card");
const stagedZipName = document.getElementById("staged-zip-name");
const stagedZipMeta = document.getElementById("staged-zip-meta");
const btnRemoveZip = document.getElementById("btn-remove-zip");

// Tab 1: Form & Action
const importDetails = document.getElementById("import-details");
const impCode = document.getElementById("imp-code");
const impTitle = document.getElementById("imp-title");
const impTime = document.getElementById("imp-time");
const impMem = document.getElementById("imp-mem");
const testsCountText = document.getElementById("tests-count-text");
const btnStartStaged = document.getElementById("btn-start-staged");

// Tab 2: Manual
const manCode = document.getElementById("man-code");
const manTitle = document.getElementById("man-title");
const manTime = document.getElementById("man-time");
const manMem = document.getElementById("man-mem");
const manInput = document.getElementById("man-input");
const manOutput = document.getElementById("man-output");
const btnCreateManual = document.getElementById("btn-create-manual");

// Tab 3: Gallery
const gallerySearch = document.getElementById("gallery-search");
const btnRefreshGallery = document.getElementById("btn-refresh-gallery");
const galleryList = document.getElementById("gallery-list");

// ---------------- Inicjalizacja ----------------
document.addEventListener("DOMContentLoaded", () => {
  setupTabs();
  setupStagingZones();
  setupActionButtons();

  checkDaemonHealth();
  loadStagedState();
  checkAutoPdfParam();
  setInterval(checkDaemonHealth, 3000);
});

async function checkAutoPdfParam() {
  const urlParams = new URLSearchParams(window.location.search);
  const autoPdf = urlParams.get("auto_pdf");
  if (!autoPdf) return;

  try {
    const res = await fetch(`${API_BASE}/api/auto-import-pdf`, {
      method: "POST",
      headers: { "Content-Type": "application/json" },
      body: JSON.stringify({ file_path: autoPdf, auto_create: false })
    });
    if (!res.ok) return;
    const data = await res.json();

    const tabImportBtn = document.querySelector('[data-tab="tab-import"]');
    if (tabImportBtn) tabImportBtn.click();

    const importPane = document.getElementById("tab-import");
    const existingBanner = document.getElementById("auto-pdf-banner");
    if (existingBanner) existingBanner.remove();

    const banner = document.createElement("div");
    banner.id = "auto-pdf-banner";
    banner.className = "auto-pdf-banner";
    const filename = data.pdf_filename || "zadanie.pdf";
    const code = (data.problem_id || "zad").toUpperCase();
    banner.innerHTML = `
      <div class="auto-pdf-header">
        <span class="auto-pdf-title">⚡ Wykryto zadanie PDF — 1 Kliknięcie</span>
        <button type="button" class="btn-card-remove" id="btn-close-pdf-banner" title="Zamknij">✕</button>
      </div>
      <div class="auto-pdf-desc">
        Plik: <strong>${escapeHtml(filename)}</strong> &bull; Zadanie: <strong>${escapeHtml(code)}</strong>
      </div>
      <button type="button" id="btn-auto-create-now" class="btn-primary w-full" style="margin-top: 6px; padding: 10px; font-weight: 700; font-size: 13px;">
        <span>⚡ Utwórz Workspace i Otwórz VS Code (1 Klik)</span>
      </button>
    `;

    importPane.prepend(banner);

    banner.querySelector("#btn-close-pdf-banner").addEventListener("click", () => banner.remove());
    banner.querySelector("#btn-auto-create-now").addEventListener("click", async () => {
      const btn = banner.querySelector("#btn-auto-create-now");
      btn.disabled = true;
      btn.innerHTML = "<span>Tworzenie workspace i otwieranie VS Code...</span>";
      try {
        const createRes = await fetch(`${API_BASE}/api/auto-import-pdf`, {
          method: "POST",
          headers: { "Content-Type": "application/json" },
          body: JSON.stringify({ file_path: autoPdf, auto_create: true })
        });
        const createData = await createRes.json();
        if (createRes.ok && createData.success) {
          showToast(`Utworzono zadanie ${createData.problem_id.toUpperCase()}! Otwarto VS Code`, "success");
          activeProblemId = createData.problem_id;
          if (activeTaskBar) activeTaskBar.classList.remove("hidden");
          if (activeTaskName) activeTaskName.textContent = `[${createData.problem_id.toUpperCase()}]`;
          banner.remove();
          renderStagedUI({ has_pdf: false, has_zip: false });
          setTimeout(() => {
            window.close();
          }, 1200);
        } else {
          showToast(createData.detail || "Błąd tworzenia zadania", "error");
          btn.disabled = false;
          btn.innerHTML = "<span>⚡ Utwórz Workspace i Otwórz VS Code (1 Klik)</span>";
        }
      } catch (err) {
        showToast("Błąd połączenia z serwerem!", "error");
        btn.disabled = false;
      }
    });

    loadStagedState();
  } catch (err) {
    console.debug("Błąd auto_pdf:", err);
  }
}

// ---------------- Nawigacja Zakładek ----------------
function setupTabs() {
  tabButtons.forEach(btn => {
    btn.addEventListener("click", () => {
      tabButtons.forEach(b => b.classList.remove("active"));
      tabPanes.forEach(p => p.classList.remove("active"));

      btn.classList.add("active");
      const targetId = btn.dataset.tab;
      const targetPane = document.getElementById(targetId);
      if (targetPane) {
        targetPane.classList.add("active");
      }

      if (targetId === "tab-gallery") {
        loadProblems();
      } else if (targetId === "tab-import") {
        loadStagedState();
      }
    });
  });
}

// ---------------- Daemon Health & Active Task ----------------
async function checkDaemonHealth() {
  try {
    const res = await fetch(`${API_BASE}/api/status`, { method: "GET" });
    if (res.ok) {
      const data = await res.json();
      daemonStatus.className = "status-pill status-online";
      statusText.textContent = "ONLINE";

      if (data.active_problem) {
        activeProblemId = data.active_problem;
        activeTaskBar.classList.remove("hidden");
        activeTaskName.textContent = `[${data.active_problem.toUpperCase()}]`;
      } else {
        activeProblemId = null;
        activeTaskBar.classList.add("hidden");
      }
    } else {
      setDaemonOffline();
    }
  } catch (err) {
    setDaemonOffline();
  }
}

function setDaemonOffline() {
  daemonStatus.className = "status-pill status-offline";
  statusText.textContent = "OFFLINE";
}

// ---------------- Dwa Osobne Obszary Stagingu (PDF + ZIP) ----------------
function setupStagingZones() {
  // 1. PDF Drag & Drop
  setupDropEvents(dropPdf, files => {
    const pdf = files.find(f => f.name.toLowerCase().endsWith(".pdf"));
    if (pdf) {
      stageFile(pdf, "pdf");
    } else {
      showToast("Upuść plik w formacie .pdf!", "error");
    }
  });

  btnBrowsePdf.addEventListener("click", e => {
    e.stopPropagation();
    fileInputPdf.click();
  });
  dropPdf.addEventListener("click", () => fileInputPdf.click());

  fileInputPdf.addEventListener("change", e => {
    if (e.target.files && e.target.files[0]) {
      stageFile(e.target.files[0], "pdf");
      fileInputPdf.value = "";
    }
  });

  btnRemovePdf.addEventListener("click", e => {
    e.stopPropagation();
    clearStagedFile("pdf");
  });

  // 2. ZIP Drag & Drop
  setupDropEvents(dropZip, files => {
    const zip = files.find(f => f.name.toLowerCase().endsWith(".zip"));
    if (zip) {
      stageFile(zip, "zip");
    } else {
      showToast("Upuść archiwum w formacie .zip!", "error");
    }
  });

  btnBrowseZip.addEventListener("click", e => {
    e.stopPropagation();
    fileInputZip.click();
  });
  dropZip.addEventListener("click", () => fileInputZip.click());

  fileInputZip.addEventListener("change", e => {
    if (e.target.files && e.target.files[0]) {
      stageFile(e.target.files[0], "zip");
      fileInputZip.value = "";
    }
  });

  btnRemoveZip.addEventListener("click", e => {
    e.stopPropagation();
    clearStagedFile("zip");
  });
}

function setupDropEvents(element, onDropFiles) {
  ["dragenter", "dragover"].forEach(event => {
    element.addEventListener(event, e => {
      e.preventDefault();
      e.stopPropagation();
      element.classList.add("dragover");
    });
  });

  ["dragleave", "drop"].forEach(event => {
    element.addEventListener(event, e => {
      e.preventDefault();
      e.stopPropagation();
      element.classList.remove("dragover");
    });
  });

  element.addEventListener("drop", e => {
    const files = Array.from(e.dataTransfer.files || []);
    if (files.length > 0) {
      onDropFiles(files);
    }
  });
}

// ---------------- Persistence & Staging API ----------------
async function loadStagedState() {
  try {
    const res = await fetch(`${API_BASE}/api/staged-state`);
    if (!res.ok) return;
    const state = await res.json();
    renderStagedUI(state);
  } catch (err) {
    console.debug("Błąd pobierania stanu stagingu:", err);
  }
}

function renderStagedUI(state) {
  const hasPdf = !!state.has_pdf;
  const hasZip = !!state.has_zip;

  // Widok PDF
  if (hasPdf) {
    dropPdf.classList.add("hidden");
    stagedPdfCard.classList.remove("hidden");
    stagedPdfName.textContent = state.pdf_filename || "zadanie.pdf";
    stagedPdfMeta.textContent = `Wczytano treść (${formatBytes(state.pdf_size)})`;
  } else {
    dropPdf.classList.remove("hidden");
    stagedPdfCard.classList.add("hidden");
  }

  // Widok ZIP
  if (hasZip) {
    dropZip.classList.add("hidden");
    stagedZipCard.classList.remove("hidden");
    stagedZipName.textContent = state.zip_filename || "testy.zip";
    stagedZipMeta.textContent = `Wykryto ${state.tests_count || 0} testów (${formatBytes(state.zip_size)})`;
  } else {
    dropZip.classList.remove("hidden");
    stagedZipCard.classList.add("hidden");
  }

  // Formularz i przycisk Rozpocznij
  if (hasPdf || hasZip) {
    importDetails.classList.remove("hidden");
    btnStartStaged.classList.remove("hidden");

    if (state.problem_id && (!impCode.value || impCode.value === "zad")) {
      impCode.value = state.problem_id;
    }
    if (state.title && !impTitle.value) {
      impTitle.value = state.title;
    }
    impTime.value = state.time_limit_sec || 1.0;
    impMem.value = state.memory_limit_mb || 128;
    testsCountText.textContent = `${state.tests_count || 0} testów wykrytych w zadaniu`;
  } else {
    importDetails.classList.add("hidden");
    btnStartStaged.classList.add("hidden");
    impCode.value = "";
    impTitle.value = "";
  }
}

async function stageFile(file, fileType) {
  const formData = new FormData();
  formData.append("file", file);
  formData.append("file_type", fileType);

  showToast(`Wczytywanie ${file.name}...`, "info");
  try {
    const res = await fetch(`${API_BASE}/api/stage-file`, {
      method: "POST",
      body: formData
    });
    if (res.ok) {
      const state = await res.json();
      renderStagedUI(state);
      showToast(`Zapisano ${file.name} w poczekalni`, "success");
    } else {
      showToast("Błąd parsowania pliku!", "error");
    }
  } catch (err) {
    showToast("Błąd połączenia z serwerem AlgoDeck!", "error");
  }
}

async function clearStagedFile(fileType) {
  try {
    const res = await fetch(`${API_BASE}/api/clear-staged`, {
      method: "POST",
      headers: { "Content-Type": "application/json" },
      body: JSON.stringify({ file_type: fileType })
    });
    if (res.ok) {
      const state = await res.json();
      renderStagedUI(state);
      showToast(`Usunięto ${fileType.toUpperCase()} z poczekalni`, "info");
    }
  } catch (err) {
    showToast("Błąd usuwania pliku z poczekalni", "error");
  }
}

// ---------------- Akcje Tworzenia Zadań ----------------
function setupActionButtons() {
  // Przycisk "Rozpocznij" (Import ze Stagingu)
  btnStartStaged.addEventListener("click", async () => {
    const pid = impCode.value.trim();
    if (!pid) {
      showToast("Podaj kod zadania!", "error");
      impCode.focus();
      return;
    }

    const payload = {
      problem_id: pid,
      title: impTitle.value.trim() || pid.toUpperCase(),
      time_limit: parseFloat(impTime.value) || 1.0,
      memory_limit: parseInt(impMem.value) || 128
    };

    btnStartStaged.disabled = true;
    btnStartStaged.innerHTML = "<span>⏳ Przygotowywanie środowiska...</span>";

    try {
      const res = await fetch(`${API_BASE}/api/start-staged`, {
        method: "POST",
        headers: { "Content-Type": "application/json" },
        body: JSON.stringify(payload)
      });

      const data = await res.json();
      if (res.ok && data.success) {
        showToast(`Utworzono '${data.problem_id}'! Otwarto VS Code`, "success");
        activeProblemId = data.problem_id;
        activeTaskBar.classList.remove("hidden");
        activeTaskName.textContent = `[${data.problem_id.toUpperCase()}]`;

        // Zresetuj widok stagingu
        renderStagedUI({ has_pdf: false, has_zip: false });
      } else {
        showToast(data.detail || "Błąd uruchamiania zadania", "error");
      }
    } catch (err) {
      showToast("Błąd połączenia z serwerem AlgoDeck!", "error");
    } finally {
      btnStartStaged.disabled = false;
      btnStartStaged.innerHTML = "<span>⚡ Rozpocznij</span>";
    }
  });

  // Ręczny Workspace
  btnCreateManual.addEventListener("click", async () => {
    const pId = manCode.value.trim();
    if (!pId) {
      showToast("Podaj nazwę zadania!", "error");
      manCode.focus();
      return;
    }

    const payload = {
      problem_id: pId,
      title: manTitle.value.trim() || pId.toUpperCase(),
      time_limit: parseFloat(manTime.value) || 1.0,
      memory_limit: parseInt(manMem.value) || 128,
      tests: []
    };

    const inContent = manInput.value;
    const outContent = manOutput.value;
    if (inContent.trim() || outContent.trim()) {
      payload.tests.push({
        id: "test_1",
        name: "Przykład 1",
        input: inContent,
        expected_output: outContent
      });
    }

    btnCreateManual.disabled = true;
    btnCreateManual.innerHTML = "<span>Tworzenie workspace...</span>";

    try {
      const res = await fetch(`${API_BASE}/api/create-manual`, {
        method: "POST",
        headers: { "Content-Type": "application/json" },
        body: JSON.stringify(payload)
      });

      const data = await res.json();
      if (res.ok && data.success) {
        showToast(`Workspace '${data.problem_id}' gotowy! Otwarto VS Code`, "success");
        activeProblemId = data.problem_id;
        activeTaskBar.classList.remove("hidden");
        activeTaskName.textContent = `[${data.problem_id.toUpperCase()}]`;

        manCode.value = "";
        manTitle.value = "";
        manInput.value = "";
        manOutput.value = "";
      } else {
        showToast(data.detail || "Błąd tworzenia zadania", "error");
      }
    } catch (err) {
      showToast("Błąd połączenia z serwerem!", "error");
    } finally {
      btnCreateManual.disabled = false;
      btnCreateManual.innerHTML = "<span>Utwórz Workspace i Otwórz VS Code</span>";
    }
  });

  // Quick VS Code button
  btnQuickVscode.addEventListener("click", async () => {
    if (activeProblemId) {
      await fetch(`${API_BASE}/api/open-vscode/${activeProblemId}`, { method: "POST" });
      showToast(`Otwarto '${activeProblemId}' w VS Code`, "info");
    }
  });

  // Galeria
  btnRefreshGallery.addEventListener("click", loadProblems);
  gallerySearch.addEventListener("input", filterGallery);
}

// ---------------- Galeria Zadań & Usuwanie ----------------
async function loadProblems() {
  galleryList.innerHTML = `<div class="loading-state">Ładowanie zadań...</div>`;
  try {
    const res = await fetch(`${API_BASE}/api/problems`);
    if (res.ok) {
      const data = await res.json();
      allProblems = data.problems || [];
      if (data.active) {
        activeProblemId = data.active;
        activeTaskBar.classList.remove("hidden");
        activeTaskName.textContent = `[${data.active.toUpperCase()}]`;
      }
      renderGallery(allProblems);
    }
  } catch (err) {
    galleryList.innerHTML = `<div class="empty-state">Błąd pobierania listy zadań</div>`;
  }
}

function filterGallery() {
  const query = gallerySearch.value.toLowerCase().trim();
  if (!query) {
    renderGallery(allProblems);
    return;
  }
  const filtered = allProblems.filter(p => {
    const pid = (p.problem_id || "").toLowerCase();
    const ptitle = (p.title || "").toLowerCase();
    return pid.includes(query) || ptitle.includes(query);
  });
  renderGallery(filtered);
}

function renderGallery(problems) {
  if (!problems || problems.length === 0) {
    galleryList.innerHTML = `<div class="empty-state">Brak zapisanych zadań</div>`;
    return;
  }

  galleryList.innerHTML = "";
  problems.forEach(p => {
    const pid = p.problem_id || "zad";
    const title = p.title || pid.toUpperCase();
    const testsCount = (p.tests || []).length;
    const isActive = (pid.toLowerCase() === (activeProblemId || "").toLowerCase());

    const card = document.createElement("div");
    card.className = `problem-card ${isActive ? "active-card" : ""}`;
    card.id = `card-prob-${pid}`;
    card.innerHTML = `
      <div class="problem-header">
        <span class="problem-code-badge">${escapeHtml(pid.toUpperCase())}</span>
        <span class="problem-title" title="${escapeHtml(title)}">${escapeHtml(title)}</span>
        <span class="problem-tests-count" id="count-prob-${pid}">${testsCount} testów</span>
      </div>
      <div class="problem-actions">
        <button class="btn-secondary btn-act-activate" title="Aktywuj i przełącz Stream Deck">Aktywuj</button>
        <button class="btn-secondary btn-act-code" title="Otwórz w edytorze">VS Code</button>
        <button class="btn-secondary btn-act-test" title="Uruchom testy">Testy</button>
        <button class="btn-secondary btn-act-add-tests" title="Dodaj paczkę testów (.zip)">+ Testy (ZIP)</button>
        <button class="btn-act-delete" title="Usuń to zadanie">Usuń</button>
      </div>
    `;

    card.querySelector(".btn-act-activate").addEventListener("click", async () => {
      await fetch(`${API_BASE}/api/set-active/${pid}`, { method: "POST" });
      activeProblemId = pid;
      activeTaskBar.classList.remove("hidden");
      activeTaskName.textContent = `[${pid.toUpperCase()}]`;
      showToast(`Przełączono na '${pid.toUpperCase()}'`, "info");
      loadProblems();
    });

    card.querySelector(".btn-act-code").addEventListener("click", async () => {
      await fetch(`${API_BASE}/api/open-vscode/${pid}`, { method: "POST" });
      showToast(`Otwarto '${pid}' w VS Code`, "info");
    });

    card.querySelector(".btn-act-test").addEventListener("click", async () => {
      showToast(`Uruchamianie testów dla '${pid}'...`, "info");
      const res = await fetch(`${API_BASE}/api/run-all/${pid}`, { method: "POST" });
      const data = await res.json();
      if (data.all_passed) {
        showToast(`Wszystkie testy zaliczone (${data.total_time_ms} ms)!`, "success");
      } else {
        showToast(`Wykryto błędy w testach (${data.summary || "WA"})`, "error");
      }
    });

    // Dodawanie testów z ZIP do istniejącego zadania
    card.querySelector(".btn-act-add-tests").addEventListener("click", () => {
      const fileInput = document.createElement("input");
      fileInput.type = "file";
      fileInput.accept = ".zip";
      fileInput.onchange = async () => {
        if (!fileInput.files || fileInput.files.length === 0) return;
        const file = fileInput.files[0];
        const formData = new FormData();
        formData.append("file", file);
        showToast(`Wgrywanie testów dla '${pid.toUpperCase()}'...`, "info");
        try {
          const res = await fetch(`${API_BASE}/api/problems/${pid}/add-tests`, {
            method: "POST",
            body: formData
          });
          const data = await res.json();
          if (res.ok && data.success) {
            showToast(`✓ Dodano ${data.tests_added} testów (łącznie: ${data.total_tests})!`, "success");
            loadProblems();
          } else {
            showToast(data.detail || "Błąd dodawania testów!", "error");
          }
        } catch (err) {
          showToast("Błąd połączenia z serwerem!", "error");
        }
      };
      fileInput.click();
    });

    // Usuwanie zadania
    card.querySelector(".btn-act-delete").addEventListener("click", async () => {
      if (!confirm(`Czy na pewno usunąć zadanie '${pid.toUpperCase()}' z workspace?`)) {
        return;
      }
      try {
        const res = await fetch(`${API_BASE}/api/problem/${pid}`, { method: "DELETE" });
        if (res.ok) {
          showToast(`Usunięto zadanie '${pid.toUpperCase()}'`, "info");
          card.remove();
          allProblems = allProblems.filter(x => (x.problem_id || "").toLowerCase() !== pid);
          if (allProblems.length === 0) {
            galleryList.innerHTML = `<div class="empty-state">Brak zapisanych zadań</div>`;
          }
          checkDaemonHealth();
        } else {
          showToast("Błąd podczas usuwania zadania!", "error");
        }
      } catch (err) {
        showToast("Błąd połączenia z serwerem!", "error");
      }
    });

    galleryList.appendChild(card);
  });
}

// ---------------- Powiadomienia Toast & Helpers ----------------
function showToast(message, type = "info") {
  const container = document.getElementById("toast-container");
  const toast = document.createElement("div");
  toast.className = `toast toast-${type}`;
  toast.textContent = message;

  container.appendChild(toast);
  setTimeout(() => {
    toast.style.opacity = "0";
    toast.style.transition = "opacity 0.3s ease";
    setTimeout(() => toast.remove(), 300);
  }, 2400);
}

function escapeHtml(str) {
  if (!str) return "";
  return str.replace(/&/g, "&amp;").replace(/</g, "&lt;").replace(/>/g, "&gt;").replace(/"/g, "&quot;");
}

function formatBytes(bytes) {
  if (!bytes) return "0 B";
  const k = 1024;
  const sizes = ["B", "KB", "MB"];
  const i = Math.floor(Math.log(bytes) / Math.log(k));
  return parseFloat((bytes / Math.pow(k, i)).toFixed(1)) + " " + sizes[i];
}
