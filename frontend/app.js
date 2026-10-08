// AlgoDeck Frontend Application Logic

let socket = null;
let currentProblemId = null;
let currentManifest = null;
let streamDeckKeys = [];

// DOM Elements
const pdfDropzone = document.getElementById('pdf-dropzone');
const pdfFileInput = document.getElementById('pdf-file-input');
const btnBrowseFile = document.getElementById('btn-browse-file');
const btnDemoTask = document.getElementById('btn-demo-task');
const pipelineStepper = document.getElementById('pipeline-stepper');
const stepperStatusMsg = document.getElementById('stepper-status-msg');

const dashboardSection = document.getElementById('dashboard-section');
const problemTitle = document.getElementById('problem-title');
const problemIdTag = document.getElementById('problem-id-tag');
const problemLimitsTag = document.getElementById('problem-limits-tag');
const problemSummaryText = document.getElementById('problem-summary-text');
const problemConstraintsText = document.getElementById('problem-constraints-text');
const testsList = document.getElementById('tests-list');
const testsCountBadge = document.getElementById('tests-count-badge');
const streamdeckGrid = document.getElementById('streamdeck-grid');
const consoleOutput = document.getElementById('console-output');

const deckStatusBadge = document.getElementById('deck-status-badge');
const deckStatusText = document.getElementById('deck-status-text');
const activeTaskPill = document.getElementById('active-task-pill');
const activeTaskName = document.getElementById('active-task-name');

const btnRunAllTests = document.getElementById('btn-run-all-tests');
const btnOpenInVscode = document.getElementById('btn-open-in-vscode');
const btnRecompile = document.getElementById('btn-recompile');
const btnClearConsole = document.getElementById('btn-clear-console');

// Settings modal elements
const btnOpenSettings = document.getElementById('btn-open-settings');
const btnCloseSettings = document.getElementById('btn-close-settings');
const settingsModal = document.getElementById('settings-modal');
const btnSaveSettings = document.getElementById('btn-save-settings');

// Web Audio API for tactile button click feedback
let audioCtx = null;
function playTactileClick() {
  try {
    if (!audioCtx) audioCtx = new (window.AudioContext || window.webkitAudioContext)();
    const osc = audioCtx.createOscillator();
    const gain = audioCtx.createGain();
    osc.type = 'triangle';
    osc.frequency.setValueAtTime(140, audioCtx.currentTime);
    osc.frequency.exponentialRampToValueAtTime(40, audioCtx.currentTime + 0.04);
    gain.gain.setValueAtTime(0.12, audioCtx.currentTime);
    gain.gain.exponentialRampToValueAtTime(0.001, audioCtx.currentTime + 0.04);
    osc.connect(gain);
    gain.connect(audioCtx.destination);
    osc.start();
    osc.stop(audioCtx.currentTime + 0.04);
  } catch (e) {
    // Audio optional
  }
}

// ============================================================================
// WEBSOCKET REAL-TIME SYNC
// ============================================================================
function initWebSocket() {
  const protocol = window.location.protocol === 'https:' ? 'wss:' : 'ws:';
  const wsUrl = `${protocol}//${window.location.host}/ws`;

  socket = new WebSocket(wsUrl);

  socket.onopen = () => {
    console.log('[AlgoDeck] WebSocket connected.');
  };

  socket.onmessage = (event) => {
    const data = JSON.parse(event.data);
    handleSocketMessage(data);
  };

  socket.onclose = () => {
    console.warn('[AlgoDeck] WebSocket closed. Retrying in 2 seconds...');
    setTimeout(initWebSocket, 2000);
  };
}

function handleSocketMessage(data) {
  switch (data.type) {
    case 'INITIAL_STATE':
      updateHardwareStatus(data.hardware_connected);
      if (data.keys) renderStreamDeckKeys(data.keys);
      if (data.active_problem) loadProblem(data.active_problem);
      break;

    case 'STREAMDECK_UPDATED':
      if (data.keys) renderStreamDeckKeys(data.keys);
      if (data.active_problem && data.active_problem !== currentProblemId) {
        loadProblem(data.active_problem);
      }
      break;

    case 'PIPELINE_STEP':
      handlePipelineStep(data);
      break;
  }
}

function updateHardwareStatus(hardwareConnected) {
  if (hardwareConnected) {
    deckStatusBadge.className = 'status-pill online';
    deckStatusText.textContent = 'Stream Deck USB Aktywny';
  } else {
    deckStatusBadge.className = 'status-pill virtual';
    deckStatusText.textContent = 'Wirtualny Deck Aktywny';
  }
}

// ============================================================================
// DRAG & DROP AND FILE UPLOAD
// ============================================================================
function setupUploadHandlers() {
  btnBrowseFile.addEventListener('click', (e) => {
    e.stopPropagation();
    pdfFileInput.click();
  });

  pdfDropzone.addEventListener('click', () => {
    pdfFileInput.click();
  });

  ['dragenter', 'dragover'].forEach(name => {
    pdfDropzone.addEventListener(name, (e) => {
      e.preventDefault();
      pdfDropzone.classList.add('dragover');
    });
  });

  ['dragleave', 'drop'].forEach(name => {
    pdfDropzone.addEventListener(name, (e) => {
      e.preventDefault();
      pdfDropzone.classList.remove('dragover');
    });
  });

  pdfDropzone.addEventListener('drop', (e) => {
    const files = e.dataTransfer.files;
    if (files.length > 0 && files[0].type === 'application/pdf') {
      uploadPDF(files[0]);
    } else {
      alert('Proszę upuścić poprawny plik PDF.');
    }
  });

  pdfFileInput.addEventListener('change', () => {
    if (pdfFileInput.files.length > 0) {
      uploadPDF(pdfFileInput.files[0]);
    }
  });

  btnDemoTask.addEventListener('click', (e) => {
    e.stopPropagation();
    loadDemoTask();
  });
}

async function uploadPDF(file) {
  pipelineStepper.classList.remove('hidden');
  resetStepper();

  const formData = new FormData();
  formData.append('file', file);

  const apiKey = localStorage.getItem('algodeck_gemini_api_key');
  if (apiKey) formData.append('api_key', apiKey);

  try {
    const response = await fetch('/api/upload-pdf', {
      method: 'POST',
      body: formData
    });

    const result = await response.json();
    if (result.success) {
      currentProblemId = result.problem_id;
      loadProblem(currentProblemId);
      logToConsole(`[AlgoDeck] Pomyślnie utworzono środowisko dla: ${result.analysis.title} (${currentProblemId})`);
    } else {
      alert(`Błąd: ${result.error || 'Nieznany błąd'}`);
    }
  } catch (err) {
    console.error(err);
    alert('Błąd podczas wysyłania pliku PDF.');
  }
}

function handlePipelineStep(stepData) {
  stepperStatusMsg.textContent = stepData.message;

  const stepMap = {
    'PDF_SAVED': 'step-pdf',
    'AI_ANALYSIS': 'step-ai',
    'WORKSPACE_GENERATION': 'step-workspace',
    'STREAMDECK_SETUP': 'step-deck',
    'COMPLETE': 'step-vscode'
  };

  const activeCardId = stepMap[stepData.step];
  if (activeCardId) {
    const card = document.getElementById(activeCardId);
    if (card) {
      card.classList.add('active');
      // Mark previous cards as done
      const allCards = document.querySelectorAll('.step-card');
      let found = false;
      allCards.forEach(c => {
        if (c.id === activeCardId) found = true;
        else if (!found) {
          c.classList.remove('active');
          c.classList.add('done');
        }
      });
    }
  }

  if (stepData.step === 'COMPLETE') {
    document.querySelectorAll('.step-card').forEach(c => c.classList.add('done'));
    setTimeout(() => {
      pipelineStepper.classList.add('hidden');
    }, 2500);
  }
}

function resetStepper() {
  document.querySelectorAll('.step-card').forEach(c => {
    c.classList.remove('active', 'done');
  });
  stepperStatusMsg.textContent = 'Rozpoczynanie...';
}

// ============================================================================
// PROBLEM LOADING & DASHBOARD
// ============================================================================
async function loadProblem(problemId) {
  try {
    const res = await fetch(`/api/problem/${problemId}`);
    if (!res.ok) return;

    const data = await res.json();
    currentProblemId = problemId;
    currentManifest = data.manifest;

    renderProblemDetails(data.manifest);
    renderTests(data.manifest.tests || []);

    // Update Navbar pill
    activeTaskPill.classList.remove('hidden');
    activeTaskName.textContent = problemId.toUpperCase();

    // Show dashboard
    dashboardSection.classList.remove('hidden');

  } catch (e) {
    console.error('Error loading problem:', e);
  }
}

function renderProblemDetails(manifest) {
  problemTitle.textContent = manifest.title || manifest.problem_id;
  problemIdTag.textContent = manifest.problem_id;
  problemLimitsTag.textContent = `⏱️ ${manifest.time_limit_sec}s | 💾 ${manifest.memory_limit_mb}MB`;
  problemSummaryText.textContent = manifest.summary || manifest.recommended_approach || 'Brak opisu.';
  problemConstraintsText.textContent = manifest.constraints || 'Brak podanych ograniczeń.';
}

function renderTests(tests) {
  testsList.innerHTML = '';
  testsCountBadge.textContent = `${tests.length} testów`;

  tests.forEach((test, idx) => {
    const item = document.createElement('div');
    item.className = 'test-item';
    item.id = `test-card-${test.id}`;

    const isEdge = test.is_edge_case;

    item.innerHTML = `
      <div class="test-info">
        <div class="test-title-row">
          <span class="test-name">${test.name}</span>
          ${isEdge ? '<span class="badge-edge">Corner Case</span>' : ''}
        </div>
        <span class="test-desc">${test.description || 'Test weryfikacyjny'}</span>
      </div>
      <div class="test-controls">
        <span id="badge-verdict-${test.id}" class="test-status-pill">BRAK</span>
        <button class="btn btn-action btn-sm" onclick="copyTestInput('${test.id}')" title="Kopiuj dane wejściowe do schowka">
          📋 Kopiuj In
        </button>
        <button class="btn btn-action btn-sm" onclick="runSingleTest('${test.id}')" title="Uruchom ten test">
          ▶ Uruchom
        </button>
      </div>
    `;

    testsList.appendChild(item);
  });
}

// ============================================================================
// STREAM DECK 15-KEY VIRTUAL HARDWARE GRID
// ============================================================================
function renderStreamDeckKeys(keys) {
  streamDeckKeys = keys;
  streamdeckGrid.innerHTML = '';

  keys.forEach((key, index) => {
    const btn = document.createElement('button');
    btn.className = `deck-key status-${key.status || 'IDLE'}`;
    btn.id = `deck-key-${index}`;
    btn.title = `Klawisz ${index + 1}: ${key.label} (${key.action})`;

    btn.innerHTML = `
      <span class="key-icon">${key.icon || '•'}</span>
      <span class="key-label">${key.label || ''}</span>
      <span class="key-subtext">${key.subtext || ''}</span>
    `;

    btn.addEventListener('click', () => {
      playTactileClick();
      pressStreamDeckKey(index);
    });

    streamdeckGrid.appendChild(btn);
  });
}

async function pressStreamDeckKey(index) {
  logToConsole(`[StreamDeck] Wciśnięto klawisz #${index}: ${streamDeckKeys[index]?.label}`);
  
  // Set immediate visual state
  const keyElem = document.getElementById(`deck-key-${index}`);
  if (keyElem) {
    keyElem.classList.add('status-RUNNING');
  }

  try {
    const res = await fetch(`/api/streamdeck/press/${index}`, { method: 'POST' });
    const result = await res.json();
    handleActionResult(result);
  } catch (err) {
    console.error(err);
  }
}

// ============================================================================
// RUNNER ACTIONS & CONSOLE
// ============================================================================
async function runSingleTest(testId) {
  logToConsole(`[Runner] Uruchamianie testu: ${testId}...`);
  try {
    const res = await fetch(`/api/run-test/${currentProblemId}/${testId}`, { method: 'POST' });
    const result = await res.json();
    handleTestResult(result);
  } catch (err) {
    console.error(err);
  }
}

async function runAllTests() {
  logToConsole(`[Runner] Uruchamianie wszystkich testów dla zadania '${currentProblemId}'...`);
  try {
    const res = await fetch(`/api/run-all/${currentProblemId}`, { method: 'POST' });
    const result = await res.json();
    handleActionResult(result);
  } catch (err) {
    console.error(err);
  }
}

async function copyTestInput(testId) {
  try {
    const res = await fetch(`/api/copy-input/${currentProblemId}/${testId}`, { method: 'POST' });
    const result = await res.json();
    if (result.success) {
      if (navigator.clipboard && result.content) {
        navigator.clipboard.writeText(result.content);
      }
      logToConsole(`[Schowek] Skopiowano wejście testu '${testId}' do schowka systemowego.`);
    }
  } catch (err) {
    console.error(err);
  }
}

function handleTestResult(result) {
  const badge = document.getElementById(`badge-verdict-${result.test_id}`);
  if (badge) {
    badge.className = `test-status-pill ${result.verdict}`;
    badge.textContent = `${result.verdict} (${result.time_ms}ms)`;
  }

  if (result.verdict === 'OK') {
    logToConsole(`✅ Test ${result.test_id}: PASSED w ${result.time_ms}ms`);
  } else if (result.verdict === 'WA') {
    logToConsole(`❌ Test ${result.test_id}: BŁĄD ODPOWIEDZI (WA) w ${result.time_ms}ms\n--- DIFF ---\n${result.diff || ''}`);
  } else if (result.verdict === 'TLE') {
    logToConsole(`⏱️ Test ${result.test_id}: PRZEKROCZONO LIMIT CZASU (TLE) > ${result.time_ms}ms`);
  } else if (result.verdict === 'RTE') {
    logToConsole(`💥 Test ${result.test_id}: BŁĄD WYKONANIA (RTE):\n${result.error || ''}`);
  }
}

function handleActionResult(result) {
  if (result.results) {
    // Run all results
    result.results.forEach(r => handleTestResult(r));
    const allPassed = result.all_passed;
    logToConsole(allPassed 
      ? `🎉 Wszystkie testy zaliczone pomyślnie! Łączny czas: ${result.total_time_ms}ms` 
      : `⚠️ Niektóre testy zakończyły się niepowodzeniem. Sprawdź diff powyżej.`
    );
  } else if (result.verdict) {
    handleTestResult(result);
  } else if (result.warnings) {
    logToConsole(`[Kompilacja] Zakończona sukcesem (${result.duration_ms}ms).\n${result.warnings}`);
  } else if (result.error) {
    logToConsole(`[Błąd Kompilacji]\n${result.error}`);
  }
}

function logToConsole(text) {
  const p = document.createElement('div');
  
  // Format diff lines
  const lines = text.split('\n');
  const formattedLines = lines.map(line => {
    if (line.startsWith('+') && !line.startsWith('+++')) {
      return `<span class="diff-add">${escapeHtml(line)}</span>`;
    } else if (line.startsWith('-') && !line.startsWith('---')) {
      return `<span class="diff-sub">${escapeHtml(line)}</span>`;
    } else if (line.startsWith('@@') || line.startsWith('---')) {
      return `<span class="diff-header">${escapeHtml(line)}</span>`;
    }
    return escapeHtml(line);
  }).join('\n');

  p.innerHTML = formattedLines;
  consoleOutput.appendChild(p);
  consoleOutput.scrollTop = consoleOutput.scrollHeight;
}

function escapeHtml(str) {
  return str.replace(/&/g, '&amp;').replace(/</g, '&lt;').replace(/>/g, '&gt;');
}

// ============================================================================
// DEMO TASK INITIALIZER (Koleje)
// ============================================================================
async function loadDemoTask() {
  logToConsole('[AlgoDeck] Ładowanie demonstracyjnego zadania olimpijskiego (Koleje)...');
  
  // Trigger synthetic upload or call demo endpoint
  const demoData = {
    problem_id: 'kol',
    title: 'Koleje (IX OI)',
    time_limit_sec: 1.0,
    memory_limit_mb: 128,
    summary: 'Klasyczne zadanie na drzewo przedziałowe (przedział-przedział) z operacją dodawania na przedziale i wyszukiwania maksimum.',
    input_format: 'W pierwszym wierszu liczby: n (liczba miast), m (pojemność pociągu), z (liczba zgłoszeń). Kolejne z wierszy to p_i, k_i, l_i.',
    output_format: 'Dla każdego zgłoszenia jedno słowo: T (gdy zaakceptowano) lub N (gdy odrzucono).',
    constraints: 'n <= 60 000, m <= 60 000, z <= 100 000',
    recommended_approach: 'Drzewo przedziałowe z leniwą propagacją (Lazy Propagation), złożoność O(z * log n).',
    tests: [
      {
        name: 'Przykład 1 (Treść zadania)',
        input: '4 6 4\n1 4 2\n1 3 2\n2 4 3\n1 2 3',
        expected_output: 'T\nT\nN\nT',
        is_edge_case: false,
        description: 'Oficjalny przykład z IX Olimpiady Informatycznej'
      },
      {
        name: 'Brzegowy: Puste pociągi & Skrajne stacje',
        input: '2 10 2\n1 2 10\n1 2 1',
        expected_output: 'T\nN',
        is_edge_case: true,
        description: 'Maksymalne zapełnienie na jednym odcinku'
      },
      {
        name: 'Brzegowy: Zgłoszenie na 0 pasażerów',
        input: '5 5 1\n1 5 0',
        expected_output: 'T',
        is_edge_case: true,
        description: 'Brak rezerwacji pasażerów'
      }
    ]
  };

  // Create workspace directly via REST API
  try {
    const res = await fetch('/api/upload-pdf', {
      method: 'POST',
      body: (() => {
        const fd = new FormData();
        const blob = new Blob(['%PDF-1.4 sample koleje'], { type: 'application/pdf' });
        fd.append('file', blob, 'koleje.pdf');
        return fd;
      })()
    });
    const result = await res.json();
    currentProblemId = result.problem_id;
    loadProblem(currentProblemId);
  } catch (e) {
    console.error(e);
  }
}

// ============================================================================
// SETTINGS MODAL & EVENT LISTENERS
// ============================================================================
function setupEventListeners() {
  btnRunAllTests.addEventListener('click', runAllTests);

  btnOpenInVscode.addEventListener('click', async () => {
    if (currentProblemId) {
      await fetch('/api/streamdeck/press/13', { method: 'POST' });
      logToConsole('[VS Code] Otwarto projekt w Visual Studio Code.');
    }
  });

  btnRecompile.addEventListener('click', async () => {
    if (currentProblemId) {
      const res = await fetch(`/api/compile/${currentProblemId}`, { method: 'POST' });
      const data = await res.json();
      handleActionResult(data);
    }
  });

  btnClearConsole.addEventListener('click', () => {
    consoleOutput.innerHTML = '<div class="console-placeholder">Konsola wyczyszczona.</div>';
  });

  btnOpenSettings.addEventListener('click', () => {
    const key = localStorage.getItem('algodeck_gemini_api_key') || '';
    document.getElementById('input-api-key').value = key;
    settingsModal.classList.remove('hidden');
  });

  btnCloseSettings.addEventListener('click', () => {
    settingsModal.classList.add('hidden');
  });

  btnSaveSettings.addEventListener('click', () => {
    const key = document.getElementById('input-api-key').value.trim();
    localStorage.setItem('algodeck_gemini_api_key', key);
    settingsModal.classList.add('hidden');
    logToConsole('[Ustawienia] Zapisano klucz Gemini API.');
  });
}

// Global initialization
document.addEventListener('DOMContentLoaded', () => {
  setupUploadHandlers();
  setupEventListeners();
  initWebSocket();
});
