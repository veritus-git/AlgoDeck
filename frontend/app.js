// Minimalistyczny Frontend AlgoDeck ze wsparciem Stream Decka i bazy zadań

let currentProblemId = null;
let savedProblems = [];

// DOM Elements
const pdfDropzone = document.getElementById('pdf-dropzone');
const pdfFileInput = document.getElementById('pdf-file-input');
const btnBrowseFile = document.getElementById('btn-browse-file');
const btnDemoTask = document.getElementById('btn-demo-task');

const navBtnPrev = document.getElementById('nav-btn-prev');
const navBtnNext = document.getElementById('nav-btn-next');
const activeTaskName = document.getElementById('active-task-name');

const engineInfo = document.getElementById('engine-info');
const engineName = document.getElementById('engine-name');

const dashboardSection = document.getElementById('dashboard-section');
const problemTitle = document.getElementById('problem-title');
const problemIdTag = document.getElementById('problem-id-tag');
const problemLimits = document.getElementById('problem-limits');

const btnTestAll = document.getElementById('btn-test-all');
const btnRun = document.getElementById('btn-run');
const btnKill = document.getElementById('btn-kill');
const btnVscode = document.getElementById('btn-vscode');

const deckActivePage = document.getElementById('deck-active-page');
const deckTaskLabel = document.getElementById('deck-task-label');

const testSummaryBadge = document.getElementById('test-summary-badge');
const testsList = document.getElementById('tests-list');
const consoleOutput = document.getElementById('console-output');

const workspacesList = document.getElementById('workspaces-list');
const workspacesCount = document.getElementById('workspaces-count');

function setupUpload() {
  btnBrowseFile.onclick = (e) => { e.stopPropagation(); pdfFileInput.click(); };
  pdfDropzone.onclick = () => pdfFileInput.click();

  pdfDropzone.ondragover = (e) => { e.preventDefault(); pdfDropzone.classList.add('dragover'); };
  pdfDropzone.ondragleave = () => pdfDropzone.classList.remove('dragover');
  pdfDropzone.ondrop = (e) => {
    e.preventDefault();
    pdfDropzone.classList.remove('dragover');
    if (e.dataTransfer.files.length > 0) uploadFile(e.dataTransfer.files[0]);
  };

  pdfFileInput.onchange = () => {
    if (pdfFileInput.files.length > 0) uploadFile(pdfFileInput.files[0]);
  };

  btnDemoTask.onclick = (e) => {
    e.stopPropagation();
    loadDemo();
  };
}

async function uploadFile(file) {
  consoleOutput.textContent = `Wysyłanie ${file.name} i błyskawiczna analiza lokalna...`;
  const fd = new FormData();
  fd.append("file", file);

  try {
    const res = await fetch("/api/upload-pdf", { method: "POST", body: fd });
    const data = await res.json();
    if (data.success) {
      currentProblemId = data.problem_id;
      await loadProblem(data.problem_id, data.analysis);
      await loadWorkspaces();
    } else {
      alert("Błąd: " + (data.error || "Nie udało się załadować"));
    }
  } catch (err) {
    console.error(err);
    alert("Błąd połączenia z serwerem.");
  }
}

async function loadDemo() {
  consoleOutput.textContent = "Pobieranie przykładowego pliku zadania (koleje.pdf)...";
  try {
    const res = await fetch("/koleje.pdf");
    const blob = await res.blob();
    const file = new File([blob], "koleje.pdf", { type: "application/pdf" });
    await uploadFile(file);
  } catch (e) {
    console.error("Błąd pobierania demo:", e);
    alert("Nie udało się pobrać pliku demo.");
  }
}

async function loadProblem(problemId, analysisData = null) {
  currentProblemId = problemId.toLowerCase();
  const upper = currentProblemId.toUpperCase();

  activeTaskName.textContent = upper;
  if (deckActivePage) deckActivePage.textContent = `STRONA: ${upper}`;
  if (deckTaskLabel) deckTaskLabel.textContent = upper;

  dashboardSection.classList.remove("hidden");

  let manifest = analysisData;
  if (!manifest) {
    try {
      const res = await fetch(`/api/problem/${currentProblemId}`);
      if (res.ok) {
        const data = await res.json();
        manifest = data.manifest;
      }
    } catch (e) {
      console.warn("Nie udało się pobrać szczegółów zadania:", e);
    }
  }

  if (manifest) {
    problemTitle.textContent = manifest.title || upper;
    problemIdTag.textContent = currentProblemId;
    if (problemLimits) {
      problemLimits.textContent = `Limit czasu: ${manifest.time_limit_sec || 1.0}s | Pamięć: ${manifest.memory_limit_mb || 128}MB`;
    }
    if (manifest.used_engine) {
      engineInfo.classList.remove("hidden");
      engineName.textContent = manifest.used_engine;
    }
    renderTests(manifest.tests || []);
  }

  consoleOutput.textContent = `Aktywne zadanie: ${upper}\nKatalog roboczy: ~/algodeck-workspace/${currentProblemId}\nPlik roboczy: ~/algodeck-workspace/${currentProblemId}/${currentProblemId}.cpp\nVS Code zmaksymalizowany na pełny ekran. Stream Deck przełączony na '${upper}'.`;
  highlightActiveCard();
}

function renderTests(tests) {
  testsList.innerHTML = "";
  tests.forEach(t => {
    const row = document.createElement("div");
    row.className = "test-row";
    row.id = `row-${t.id}`;
    row.innerHTML = `
      <div class="test-row-info">
        <strong>${t.name}</strong>
      </div>
      <div class="test-row-actions">
        <span class="badge" id="badge-${t.id}">NIEURUCHOMIONY</span>
        <button class="btn btn-sm" onclick="copyInput('${t.id}')">Kopiuj In</button>
      </div>
    `;
    testsList.appendChild(row);
  });
}

// 1 KLIKNIĘCIE: TESTUJ WSZYSTKO (KOMPILUJ, ODPAL, DIFF)
async function testAll() {
  if (!currentProblemId) return;
  consoleOutput.textContent = "Kompilacja g++ -O3 i uruchamianie oficjalnych testów...";

  try {
    const res = await fetch(`/api/run-all/${currentProblemId}`, { method: "POST" });
    const data = await res.json();

    if (data.verdict === "CE") {
      testSummaryBadge.className = "tag tag-red";
      testSummaryBadge.textContent = "BŁĄD KOMPILACJI (CE)";
      consoleOutput.textContent = `Błąd kompilacji:\n${data.compile_error}`;
      return;
    }

    let allOk = true;
    let logLines = [];

    (data.results || []).forEach(r => {
      const badge = document.getElementById(`badge-${r.test_id}`);
      if (badge) {
        badge.className = `badge badge-${r.verdict}`;
        badge.textContent = `${r.verdict} (${r.time_ms} ms)`;
      }

      if (r.verdict === "OK" || r.verdict === "OK (RUN)") {
        logLines.push(`✅ [${r.name}] OK w ${r.time_ms} ms`);
      } else if (r.verdict === "WA") {
        allOk = false;
        logLines.push(`❌ [${r.name}] BŁĄD ODPOWIEDZI (WA) w ${r.time_ms} ms`);
        if (r.diff) logLines.push(`--- DIFF ---\n${r.diff}\n------------`);
      } else {
        allOk = false;
        logLines.push(`⚠️ [${r.name}] ${r.verdict}: ${r.error || r.details || ''}`);
      }
    });

    if (allOk) {
      testSummaryBadge.className = "tag tag-green";
      testSummaryBadge.textContent = `WSZYSTKO OK (${data.total_time_ms} ms)`;
    } else {
      testSummaryBadge.className = "tag tag-red";
      testSummaryBadge.textContent = "ZNALEZIONO BŁĘDY";
    }

    consoleOutput.textContent = logLines.join("\n");
  } catch (e) {
    console.error(e);
    consoleOutput.textContent = "Błąd wykonania testów.";
  }
}

async function copyInput(testId) {
  const res = await fetch(`/api/copy-input/${currentProblemId}/${testId}`, { method: "POST" });
  const data = await res.json();
  if (data.content && navigator.clipboard) {
    navigator.clipboard.writeText(data.content);
    consoleOutput.textContent = `Skopiowano dane wejściowe testu '${testId}' do schowka.`;
  }
}

// Obsługa akcji Stream Decka
async function handleStreamDeckAction(action) {
  if (!currentProblemId && action !== 'prev' && action !== 'next') return;

  if (action === 'vscode') {
    await fetch(`/api/open-vscode/${currentProblemId}`, { method: "POST" });
    consoleOutput.textContent = "💻 VS Code uruchomiony / zmaksymalizowany na pełny ekran.";
  } else if (action === 'test') {
    testAll();
  } else if (action === 'play') {
    await fetch(`/api/streamdeck/press/7`, { method: "POST" });
    consoleOutput.textContent = "▶️ Uruchomiono program w terminalu VS Code...";
  } else if (action === 'kill') {
    await fetch(`/api/kill/${currentProblemId}`, { method: "POST" });
    consoleOutput.textContent = "🛑 Zatrzymano proces (KILL).";
  } else if (action === 'menu') {
    await fetch(`/api/streamdeck/menu`, { method: "POST" });
    consoleOutput.textContent = "📂 Otwarto submenu zadań na Stream Decku.";
  } else if (action === 'prev') {
    await switchTask('prev');
  } else if (action === 'next') {
    await switchTask('next');
  }
}

// Przełączanie między zapisanymi zadaniami
async function switchTask(direction) {
  consoleOutput.textContent = `Przełączanie zadania (${direction})...`;
  await fetch(`/api/switch-task/${direction}`, { method: "POST" });
  
  // Odczekaj chwilę na wykonanie skryptu i pobierz aktualny stan
  setTimeout(async () => {
    await loadWorkspaces();
    const res = await fetch("/api/problems");
    const data = await res.json();
    if (data.active) {
      await loadProblem(data.active);
    }
  }, 250);
}

// Aktywowanie konkretnego zadania
async function activateProblem(probId) {
  consoleOutput.textContent = `Otwieranie zadania ${probId.toUpperCase()}...`;
  await fetch(`/api/set-active/${probId}`, { method: "POST" });
  await loadProblem(probId);
  highlightActiveCard();
}

function highlightActiveCard() {
  document.querySelectorAll(".workspace-card").forEach(c => {
    if (c.getAttribute("data-id") === currentProblemId) {
      c.classList.add("active");
    } else {
      c.classList.remove("active");
    }
  });
}

// Ładowanie listy wszystkich zadań z workspace
async function loadWorkspaces() {
  try {
    const res = await fetch("/api/problems");
    const data = await res.json();
    savedProblems = data.problems || [];
    workspacesCount.textContent = `${savedProblems.length} ${savedProblems.length === 1 ? 'zadanie' : 'zadań'}`;

    workspacesList.innerHTML = "";
    if (savedProblems.length === 0) {
      workspacesList.innerHTML = `<div class="empty-hint" style="color:#64748b; font-size:0.9rem;">Brak zadań w katalogu ~/algodeck-workspace. Upuść plik PDF powyżej, aby dodać pierwsze zadanie.</div>`;
      return;
    }

    savedProblems.forEach(p => {
      const pid = (p.problem_id || "zad").toLowerCase();
      const card = document.createElement("div");
      card.className = `workspace-card ${pid === currentProblemId ? 'active' : ''}`;
      card.setAttribute("data-id", pid);
      card.innerHTML = `
        <div class="workspace-card-header">
          <span class="tag tag-cyan">${pid}</span>
          <span class="workspace-card-info">${(p.tests || []).length} testów</span>
        </div>
        <div class="workspace-card-title">${p.title || pid}</div>
        <div class="workspace-card-info">~/algodeck-workspace/${pid}/${pid}.cpp</div>
        <div class="workspace-card-actions">
          <button class="btn btn-sm btn-primary" onclick="activateProblem('${pid}')">
            📂 Otwórz (VS Code + Deck)
          </button>
        </div>
      `;
      workspacesList.appendChild(card);
    });

    if (data.active && (!currentProblemId || currentProblemId !== data.active)) {
      await loadProblem(data.active);
    }
  } catch (e) {
    console.error("Błąd ładowania workspaces:", e);
  }
}

document.addEventListener("DOMContentLoaded", async () => {
  setupUpload();
  btnTestAll.onclick = testAll;
  btnRun.onclick = () => handleStreamDeckAction('play');
  btnKill.onclick = () => handleStreamDeckAction('kill');
  btnVscode.onclick = () => handleStreamDeckAction('vscode');

  navBtnPrev.onclick = () => switchTask('prev');
  navBtnNext.onclick = () => switchTask('next');

  await loadWorkspaces();
});
