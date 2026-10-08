// Minimalistyczny Frontend AlgoDeck

let currentProblemId = null;

const pdfDropzone = document.getElementById('pdf-dropzone');
const pdfFileInput = document.getElementById('pdf-file-input');
const btnBrowseFile = document.getElementById('btn-browse-file');
const btnDemoTask = document.getElementById('btn-demo-task');

const engineInfo = document.getElementById('engine-info');
const engineName = document.getElementById('engine-name');

const dashboardSection = document.getElementById('dashboard-section');
const problemTitle = document.getElementById('problem-title');
const problemIdTag = document.getElementById('problem-id-tag');
const activeTaskPill = document.getElementById('active-task-pill');
const activeTaskName = document.getElementById('active-task-name');

const btnTestAll = document.getElementById('btn-test-all');
const btnRun = document.getElementById('btn-run');
const btnKill = document.getElementById('btn-kill');
const btnVscode = document.getElementById('btn-vscode');
const btnBackMain = document.getElementById('btn-back-main');

const testSummaryBadge = document.getElementById('test-summary-badge');
const testsList = document.getElementById('tests-list');
const consoleOutput = document.getElementById('console-output');
const deckStatusTest = document.getElementById('deck-status-test');

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
  consoleOutput.textContent = "Wysyłanie pliku i analiza treści...";
  const fd = new FormData();
  fd.append("file", file);

  try {
    const res = await fetch("/api/upload-pdf", { method: "POST", body: fd });
    const data = await res.json();
    if (data.success) {
      currentProblemId = data.problem_id;
      loadProblem(data.problem_id, data.analysis);
    } else {
      alert("Błąd: " + (data.error || "Nie udało się załadować"));
    }
  } catch (err) {
    console.error(err);
    alert("Błąd połączenia z serwerem.");
  }
}

async function loadDemo() {
  const blob = new Blob(["%PDF-1.4 demo"], { type: "application/pdf" });
  const file = new File([blob], "koleje.pdf", { type: "application/pdf" });
  await uploadFile(file);
}

async function loadProblem(problemId, analysisData = null) {
  currentProblemId = problemId;
  activeTaskPill.classList.remove("hidden");
  activeTaskName.textContent = problemId.toUpperCase();
  dashboardSection.classList.remove("hidden");

  let manifest = analysisData;
  if (!manifest) {
    const res = await fetch(`/api/problem/${problemId}`);
    const data = await res.json();
    manifest = data.manifest;
  }

  problemTitle.textContent = manifest.title || problemId;
  problemIdTag.textContent = problemId;

  if (manifest.used_engine) {
    engineInfo.classList.remove("hidden");
    engineName.textContent = manifest.used_engine;
  }

  renderTests(manifest.tests || []);
  consoleOutput.textContent = `Zadanie ${problemId} gotowe.\nKatalog roboczy: ~/algodeck-workspace/${problemId}\nSzablon C++ wklejony. VS Code otwarty.\nStrona na Stream Decku przełączona na '${problemId.toUpperCase()}'.`;
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
  consoleOutput.textContent = "Kompilacja g++ -O3 i uruchamianie testów...";
  deckStatusTest.textContent = "RUN...";

  try {
    const res = await fetch(`/api/run-all/${currentProblemId}`, { method: "POST" });
    const data = await res.json();

    if (data.verdict === "CE") {
      testSummaryBadge.className = "tag tag-red";
      testSummaryBadge.textContent = "BŁĄD KOMPILACJI (CE)";
      deckStatusTest.textContent = "CE";
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
      deckStatusTest.textContent = "OK";
    } else {
      testSummaryBadge.className = "tag tag-red";
      testSummaryBadge.textContent = "ZNALEZIONO BŁĘDY";
      deckStatusTest.textContent = "WA";
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

async function pressStreamDeckKey(idx) {
  if (idx === 0) {
    testAll();
  } else if (idx === 1) {
    fetch(`/api/streamdeck/press/1`, { method: "POST" });
    consoleOutput.textContent = "Uruchomiono program (run.sh)...";
  } else if (idx === 2) {
    fetch(`/api/kill/${currentProblemId}`, { method: "POST" });
    consoleOutput.textContent = "🛑 Wysłano sygnał KILL do procesu.";
  } else if (idx === 3) {
    fetch(`/api/streamdeck/press/3`, { method: "POST" });
    consoleOutput.textContent = "Otwarto w VS Code.";
  } else if (idx === 4) {
    fetch(`/api/streamdeck/press/4`, { method: "POST" });
    consoleOutput.textContent = "Przełączono Stream Deck na stronę Main.";
  }
}

document.addEventListener("DOMContentLoaded", () => {
  setupUpload();
  btnTestAll.onclick = testAll;
  btnRun.onclick = () => pressStreamDeckKey(1);
  btnKill.onclick = () => pressStreamDeckKey(2);
  btnVscode.onclick = () => pressStreamDeckKey(3);
  btnBackMain.onclick = () => pressStreamDeckKey(4);
});
