// ==============================================================================
// AlgoDeck ⚡ Architecture & Settings Controller v2.0
// ==============================================================================

const API_BASE = "http://127.0.0.1:8080";

const elements = {
  // Status & Header
  statusBadge: document.getElementById("status-badge"),
  statusText: document.getElementById("status-text"),
  btnClose: document.getElementById("btn-close-settings"),
  btnTestConn: document.getElementById("btn-test-connection"),
  btnSave: document.getElementById("btn-save-settings"),
  saveIcon: document.getElementById("save-icon"),
  saveText: document.getElementById("save-text"),
  saveStatusMsg: document.getElementById("save-status-msg"),

  // Workspace
  workspaceDir: document.getElementById("cfg-workspace-dir"),
  btnResetWs: document.getElementById("btn-reset-ws"),

  // VS Code
  vscodeMode: document.getElementById("cfg-vscode-mode"),
  splitPdf: document.getElementById("cfg-split-pdf"),
  splitOptionsBox: document.getElementById("split-pdf-options"),
  pdfPosition: document.getElementById("cfg-pdf-position"),
  splitRatio: document.getElementById("cfg-split-ratio"),
  ratioLeftSeg: document.getElementById("ratio-left-seg"),
  ratioRightSeg: document.getElementById("ratio-right-seg"),
  ratioLeftLabel: document.getElementById("ratio-left-label"),
  ratioRightLabel: document.getElementById("ratio-right-label"),
  splitRatioValLabel: document.getElementById("split-ratio-val-label"),

  // Compiler & Flags
  compiler: document.getElementById("cfg-compiler"),
  cppStd: document.getElementById("cfg-cpp-std"),
  sanitizer: document.getElementById("cfg-sanitizer"),
  releaseFlags: document.getElementById("cfg-release-flags"),
  debugFlags: document.getElementById("cfg-debug-flags"),
  timeLimit: document.getElementById("cfg-time-limit"),
  memoryLimit: document.getElementById("cfg-memory-limit"),

  // Stream Deck Hardware
  sdStatusBox: document.getElementById("sd-status-box"),
  sdConnHeader: document.getElementById("sd-conn-header"),
  sdBadge: document.getElementById("sd-badge"),
  sdConnDesc: document.getElementById("sd-conn-desc"),
  btnRefreshSd: document.getElementById("btn-refresh-sd"),
  sdBrightness: document.getElementById("cfg-sd-brightness"),
  brightnessValLabel: document.getElementById("brightness-val-label"),
  virtualGrid: document.getElementById("virtual-grid-15"),
  sdActiveModeLabel: document.getElementById("sd-active-mode-label"),
  btnSdIdle: document.getElementById("btn-sd-idle"),
  btnSdTestScreen: document.getElementById("btn-sd-test-screen"),
  btnSdBlank: document.getElementById("btn-sd-blank"),

  // Browser Automation
  autoDetect: document.getElementById("cfg-auto-detect"),
  notifications: document.getElementById("cfg-notifications")
};

let currentSettings = {
  workspace_dir: "",
  vscode_mode: "auto",
  open_pdf_in_vscode: true,
  cxx_compiler: "g++",
  cpp_standard: "c++20",
  sanitize_enabled: false,
  cxx_release_flags: "-O3 -Wall -Wextra",
  cxx_debug_flags: "-O0 -g -fsanitize=address,undefined -Wall -Wextra",
  default_time_limit: 1.0,
  default_memory_limit: 128,
  streamdeck_brightness: 85,
  pdf_position: "right",
  editor_split_ratio: 60,
  auto_detect_downloads: true,
  notifications: true
};

let brightnessDebounceTimer = null;
let pollStreamDeckTimer = null;

// ==============================================================================
// 1. Connection & Status
// ==============================================================================

async function checkConnection() {
  try {
    const res = await fetch(`${API_BASE}/api/status`, { cache: "no-store" });
    if (res.ok) {
      const data = await res.json();
      elements.statusBadge.className = "status-pill online";
      const sdTxt = data.streamdeck_connected ? " • Stream Deck OK" : " • Stream Deck: odłączony";
      elements.statusText.textContent = `Online :8080 (v${data.version})${sdTxt}`;
      updateStreamDeckStatus(data.streamdeck_connected);
      return true;
    }
  } catch (_) {}

  elements.statusBadge.className = "status-pill offline";
  elements.statusText.textContent = "Offline (Silnik nie jest uruchomiony)";
  updateStreamDeckStatus(false);
  return false;
}

function updateStreamDeckStatus(connected) {
  if (connected) {
    elements.sdBadge.className = "sd-badge badge-green";
    elements.sdBadge.textContent = "POŁĄCZONO (USB HID)";
    elements.sdConnHeader.textContent = "Elgato Stream Deck 15-Key: AKTYWNY";
    elements.sdConnDesc.textContent = "Urządzenie podłączone bezpośrednio przez USB. Gotowe do sterowania zadaniami olimpijskimi.";
  } else {
    elements.sdBadge.className = "sd-badge badge-gray";
    elements.sdBadge.textContent = "ODŁĄCZONY";
    elements.sdConnHeader.textContent = "Elgato Stream Deck: Oczekiwanie na USB";
    elements.sdConnDesc.textContent = "Brak wykrytego urządzenia USB HID. Po podłączeniu Stream Deck zostanie automatycznie zainicjalizowany.";
  }
}

// ==============================================================================
// 2. Load & Apply Settings
// ==============================================================================

async function loadSettings() {
  const isOnline = await checkConnection();

  if (isOnline) {
    try {
      const res = await fetch(`${API_BASE}/api/settings`, { cache: "no-store" });
      if (res.ok) {
        const backendSettings = await res.json();
        currentSettings = { ...currentSettings, ...backendSettings };
        applySettingsToForm(currentSettings);

        if (typeof chrome !== "undefined" && chrome.storage && chrome.storage.local) {
          chrome.storage.local.set({ algodeck_settings: currentSettings });
        }
        await refreshStreamDeckVirtualGrid();
        return;
      }
    } catch (e) {
      console.warn("Błąd pobierania ustawień z backendu:", e);
    }
  }

  // Fallback z chrome.storage.local
  if (typeof chrome !== "undefined" && chrome.storage && chrome.storage.local) {
    chrome.storage.local.get(["algodeck_settings", "auto_detect_downloads"], (data) => {
      if (data.algodeck_settings) {
        currentSettings = { ...currentSettings, ...data.algodeck_settings };
      }
      if (data.auto_detect_downloads !== undefined) {
        currentSettings.auto_detect_downloads = data.auto_detect_downloads;
      }
      applySettingsToForm(currentSettings);
    });
  } else {
    applySettingsToForm(currentSettings);
  }

  renderDefaultVirtualGrid();
}

function applySettingsToForm(s) {
  if (s.workspace_dir) elements.workspaceDir.value = s.workspace_dir;
  if (s.vscode_mode) elements.vscodeMode.value = s.vscode_mode;
  if (s.open_pdf_in_vscode !== undefined) elements.splitPdf.checked = Boolean(s.open_pdf_in_vscode);
  if (s.cxx_compiler) elements.compiler.value = s.cxx_compiler;
  if (s.cpp_standard) elements.cppStd.value = s.cpp_standard;
  if (s.sanitize_enabled !== undefined) elements.sanitizer.checked = Boolean(s.sanitize_enabled);
  if (s.cxx_release_flags) elements.releaseFlags.value = s.cxx_release_flags;
  if (s.cxx_debug_flags) elements.debugFlags.value = s.cxx_debug_flags;
  if (s.default_time_limit) elements.timeLimit.value = s.default_time_limit;
  if (s.default_memory_limit) elements.memoryLimit.value = s.default_memory_limit;
  if (s.streamdeck_brightness !== undefined) {
    elements.sdBrightness.value = s.streamdeck_brightness;
    elements.brightnessValLabel.textContent = `${s.streamdeck_brightness}%`;
  }
  if (s.pdf_position) elements.pdfPosition.value = s.pdf_position;
  if (s.editor_split_ratio) elements.splitRatio.value = s.editor_split_ratio;
  if (s.auto_detect_downloads !== undefined) elements.autoDetect.checked = Boolean(s.auto_detect_downloads);
  if (s.notifications !== undefined) elements.notifications.checked = Boolean(s.notifications);
  updateSplitRatioVisual();
}

function updateSplitRatioVisual() {
  const isSplit = elements.splitPdf.checked;
  if (elements.splitOptionsBox) {
    if (isSplit) {
      elements.splitOptionsBox.classList.remove("hidden");
    } else {
      elements.splitOptionsBox.classList.add("hidden");
    }
  }

  const ratio = parseInt(elements.splitRatio.value, 10) || 60;
  const isPdfLeft = elements.pdfPosition.value === "left";
  const codeRatio = ratio;
  const pdfRatio = 100 - ratio;

  if (isPdfLeft) {
    elements.ratioLeftSeg.style.width = `${pdfRatio}%`;
    elements.ratioRightSeg.style.width = `${codeRatio}%`;
    elements.ratioLeftLabel.textContent = `📄 PDF ${pdfRatio}%`;
    elements.ratioRightLabel.textContent = `💻 Kod ${codeRatio}%`;
    elements.splitRatioValLabel.textContent = `${pdfRatio}/${codeRatio}`;
  } else {
    elements.ratioLeftSeg.style.width = `${codeRatio}%`;
    elements.ratioRightSeg.style.width = `${pdfRatio}%`;
    elements.ratioLeftLabel.textContent = `💻 Kod ${codeRatio}%`;
    elements.ratioRightLabel.textContent = `📄 PDF ${pdfRatio}%`;
    elements.splitRatioValLabel.textContent = `${codeRatio}/${pdfRatio}`;
  }
}

function collectSettingsFromForm() {
  return {
    ...currentSettings,
    workspace_dir: elements.workspaceDir.value.trim(),
    vscode_mode: elements.vscodeMode.value,
    open_pdf_in_vscode: elements.splitPdf.checked,
    pdf_position: elements.pdfPosition.value,
    editor_split_ratio: parseInt(elements.splitRatio.value, 10) || 60,
    cxx_compiler: elements.compiler.value,
    cpp_standard: elements.cppStd.value,
    sanitize_enabled: elements.sanitizer.checked,
    cxx_release_flags: elements.releaseFlags.value.trim(),
    cxx_debug_flags: elements.debugFlags.value.trim(),
    default_time_limit: parseFloat(elements.timeLimit.value) || 1.0,
    default_memory_limit: parseInt(elements.memoryLimit.value, 10) || 128,
    streamdeck_brightness: parseInt(elements.sdBrightness.value, 10) || 85,
    auto_detect_downloads: elements.autoDetect.checked,
    notifications: elements.notifications.checked
  };
}

// ==============================================================================
// 3. Save Settings
// ==============================================================================

async function saveSettings() {
  const newSettings = collectSettingsFromForm();
  elements.btnSave.disabled = true;
  elements.saveText.textContent = "Zapisywanie...";
  elements.saveIcon.textContent = "⏳";
  elements.saveStatusMsg.textContent = "";

  // 1. Zapisz w pamięci przeglądarki
  if (typeof chrome !== "undefined" && chrome.storage && chrome.storage.local) {
    chrome.storage.local.set({
      algodeck_settings: newSettings,
      auto_detect_downloads: newSettings.auto_detect_downloads
    });
  }

  // 2. Wyślij do backendu AlgoDeck (POST /api/settings)
  let savedToBackend = false;
  try {
    const res = await fetch(`${API_BASE}/api/settings`, {
      method: "POST",
      headers: { "Content-Type": "application/json" },
      body: JSON.stringify(newSettings)
    });
    if (res.ok) {
      savedToBackend = true;
      currentSettings = newSettings;
    }
  } catch (_) {}

  elements.btnSave.disabled = false;
  elements.saveIcon.textContent = "💾";
  elements.saveText.textContent = "Zapisz Ustawienia";

  if (savedToBackend) {
    elements.saveStatusMsg.textContent = "✓ Konfiguracja została pomyślnie zapisana i zsynchronizowana!";
    elements.saveStatusMsg.className = "save-feedback-text success";
  } else {
    elements.saveStatusMsg.textContent = "Zapisano lokalnie w przeglądarce (Backend AlgoDeck jest offline)";
    elements.saveStatusMsg.className = "save-feedback-text";
  }

  setTimeout(() => {
    elements.saveStatusMsg.textContent = "";
  }, 4000);
}

// ==============================================================================
// 4. Stream Deck Controls & Virtual Grid
// ==============================================================================

function renderDefaultVirtualGrid() {
  elements.virtualGrid.innerHTML = "";
  const layout = [
    { id: 0, icon: "A", label: "CODE", cls: "key-letter" },
    { id: 1, icon: "L", label: "CODE", cls: "key-letter" },
    { id: 2, icon: "G", label: "CODE", cls: "key-letter" },
    { id: 3, icon: "+", label: "NEW", cls: "" },
    { id: 4, icon: "⚙", label: "SET", cls: "" },
    { id: 5, icon: "✓", label: "TEST", cls: "key-action-test" },
    { id: 6, icon: "▶", label: "RUN", cls: "key-action-run" },
    { id: 7, icon: "⏹", label: "KILL", cls: "key-action-kill" },
    { id: 8, icon: "⊞", label: "MENU", cls: "key-action-nav" },
    { id: 9, icon: "=", label: "ALL", cls: "" },
    { id: 10, icon: "◀", label: "PREV", cls: "" },
    { id: 11, icon: "▶", label: "NEXT", cls: "" },
    { id: 12, icon: "1/4", label: "CURR", cls: "" },
    { id: 13, icon: "OK", label: "0ms", cls: "key-action-test" },
    { id: 14, icon: "128M", label: "MEM", cls: "" }
  ];

  layout.forEach((k) => {
    const el = document.createElement("div");
    el.className = `v-key ${k.cls}`;
    el.title = `Klawisz ${k.id}: ${k.label} (Kliknij, aby wysłać sygnał)`;
    el.innerHTML = `
      <span class="v-key-icon">${k.icon}</span>
      <span class="v-key-label">${k.label}</span>
    `;
    el.addEventListener("click", () => pressKey(k.id));
    elements.virtualGrid.appendChild(el);
  });
}

function resolveKeyVisual(k) {
  let icon = "●";
  let label = `K${k.id}`;
  let cls = "";

  const name = k.icon_name || "";
  const act = k.action || "";

  if (name.includes("idle_logo_algo")) {
    icon = "⚡";
    label = "ALGO";
    cls = "key-letter";
  } else if (name.includes("idle_logo_bolt")) {
    icon = "⚡";
    label = "BOLT";
    cls = "key-letter";
  } else if (name.includes("idle_logo_deck")) {
    icon = "⚡";
    label = "DECK";
    cls = "key-letter";
  } else if (name.startsWith("letter_")) {
    const char = name.replace("letter_", "").replace(".png", "").toUpperCase();
    icon = char;
    label = "CODE";
    cls = "key-letter";
  } else if (name.includes("equals")) {
    icon = "=";
    label = "CODE";
    cls = "key-letter";
  } else if (name.includes("plus") || act === "new_task") {
    icon = "+";
    label = "NOWE";
    cls = "";
  } else if (act === "run_all" || name.includes("test")) {
    icon = "✓";
    label = k.custom_label || "TEST";
    cls = "key-action-test";
  } else if (act === "run_interactive" || name.includes("play") || name.includes("run")) {
    icon = "▶";
    label = "RUN";
    cls = "key-action-run";
  } else if (act === "kill_process" || name.includes("kill")) {
    icon = "⏹";
    label = "KILL";
    cls = "key-action-kill";
  } else if (act === "open_menu" || name.includes("task_sec") || name.includes("menu")) {
    icon = "⊞";
    label = "MENU";
    cls = "key-action-nav";
  } else if (act === "open_settings" || name.includes("settings")) {
    icon = "⚙";
    label = "SET";
    cls = "";
  } else if (name.includes("arrow_left") || act === "prev_task") {
    icon = "◀";
    label = "PREV";
    cls = "";
  } else if (name.includes("arrow_right") || act === "next_task") {
    icon = "▶";
    label = "NEXT";
    cls = "";
  } else if (k.custom_label) {
    icon = k.custom_label;
    label = "TASK";
    cls = "key-action-test";
  } else if (!act && !name) {
    icon = "·";
    label = "OFF";
    cls = "";
  }

  return { icon, label, cls };
}

async function refreshStreamDeckVirtualGrid() {
  try {
    const res = await fetch(`${API_BASE}/api/streamdeck/state`, { cache: "no-store" });
    if (res.ok) {
      const state = await res.json();
      updateStreamDeckStatus(state.is_hardware_connected);
      elements.sdActiveModeLabel.textContent = state.active_problem_id 
        ? `ZADANIE: ${state.active_problem_id.toUpperCase()}` 
        : "TRYB: IDLE";

      if (state.keys && state.keys.length === 15) {
        elements.virtualGrid.innerHTML = "";
        state.keys.forEach((k) => {
          const vis = resolveKeyVisual(k);
          const el = document.createElement("div");
          el.className = `v-key ${vis.cls}`;
          el.title = `Klawisz ${k.id}: ${k.action || "brak akcji"} (Kliknij, aby wysłać sygnał)`;
          el.innerHTML = `
            <span class="v-key-icon">${vis.icon}</span>
            <span class="v-key-label">${vis.label}</span>
          `;
          el.addEventListener("click", () => pressKey(k.id));
          elements.virtualGrid.appendChild(el);
        });
      }
    }
  } catch (_) {}
}

async function pressKey(keyId) {
  try {
    await fetch(`${API_BASE}/api/streamdeck/press/${keyId}`, { method: "POST" });
    setTimeout(refreshStreamDeckVirtualGrid, 150);
  } catch (_) {}
}

function handleBrightnessChange(val) {
  elements.brightnessValLabel.textContent = `${val}%`;
  clearTimeout(brightnessDebounceTimer);
  brightnessDebounceTimer = setTimeout(async () => {
    try {
      await fetch(`${API_BASE}/api/streamdeck/brightness`, {
        method: "POST",
        headers: { "Content-Type": "application/json" },
        body: JSON.stringify({ brightness: parseInt(val, 10) })
      });
    } catch (_) {}
  }, 120);
}

// ==============================================================================
// 5. Event Listeners
// ==============================================================================

document.addEventListener("DOMContentLoaded", () => {
  loadSettings();

  // Zapis
  elements.btnSave.addEventListener("click", saveSettings);

  // Weryfikacja połączenia
  elements.btnTestConn.addEventListener("click", async () => {
    elements.saveStatusMsg.textContent = "Weryfikacja komunikacji z silnikiem...";
    elements.saveStatusMsg.className = "save-feedback-text";
    const ok = await checkConnection();
    elements.saveStatusMsg.textContent = ok 
      ? "✓ Pomyślnie połączono z demonem AlgoDeck na porcie 8080!" 
      : "Brak odpowiedzi silnika. Upewnij się, że 'algodeck start' został uruchomiony.";
    elements.saveStatusMsg.className = ok ? "save-feedback-text success" : "save-feedback-text error";
    setTimeout(() => { elements.saveStatusMsg.textContent = ""; }, 3500);
  });

  // Reset ścieżki workspace
  elements.btnResetWs.addEventListener("click", () => {
    const isWin = navigator.userAgent.includes("Windows");
    elements.workspaceDir.value = isWin ? "C:\\AlgoDeckWorkspace" : "/home/linux/AlgoDeckWorkspace";
  });

  // Zmiana proporcji podziału ekranu PDF / Kod
  elements.splitPdf.addEventListener("change", updateSplitRatioVisual);
  elements.pdfPosition.addEventListener("change", updateSplitRatioVisual);
  elements.splitRatio.addEventListener("input", updateSplitRatioVisual);

  // Jasność Stream Decka
  elements.sdBrightness.addEventListener("input", (e) => {
    handleBrightnessChange(e.target.value);
  });

  // Odśwież status Stream Decka
  elements.btnRefreshSd.addEventListener("click", async () => {
    await checkConnection();
    await refreshStreamDeckVirtualGrid();
  });

  // Przełącz na ekran Idle
  elements.btnSdIdle.addEventListener("click", async () => {
    try {
      await fetch(`${API_BASE}/api/exit`, { method: "POST" });
      setTimeout(refreshStreamDeckVirtualGrid, 200);
    } catch (_) {}
  });

  // Symuluj ekran zadania (Test)
  elements.btnSdTestScreen.addEventListener("click", async () => {
    try {
      await fetch(`${API_BASE}/api/launch`, {
        method: "POST",
        headers: { "Content-Type": "application/json" },
        body: JSON.stringify({ problem_id: "test" })
      });
      setTimeout(refreshStreamDeckVirtualGrid, 200);
    } catch (_) {}
  });

  // Wyczyść / wygaś Stream Deck
  elements.btnSdBlank.addEventListener("click", async () => {
    try {
      await fetch(`${API_BASE}/api/exit`, { method: "POST" });
      setTimeout(refreshStreamDeckVirtualGrid, 200);
    } catch (_) {}
  });

  // Zamknij okno / kartę
  elements.btnClose.addEventListener("click", () => {
    if (typeof chrome !== "undefined" && chrome.tabs && chrome.tabs.getCurrent) {
      chrome.tabs.getCurrent((tab) => {
        if (tab) chrome.tabs.remove(tab.id);
        else window.close();
      });
    } else {
      window.close();
    }
  });

  // Okresowe odświeżanie stanu Stream Decka co 3 sekundy
  pollStreamDeckTimer = setInterval(() => {
    refreshStreamDeckVirtualGrid();
  }, 3000);
});
