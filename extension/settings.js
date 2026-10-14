// AlgoDeck ⚡ Settings Controller
const API_BASE = "http://127.0.0.1:8080";

const elements = {
  statusBadge: document.getElementById("status-badge"),
  statusText: document.getElementById("status-text"),
  btnClose: document.getElementById("btn-close-settings"),
  btnTestConn: document.getElementById("btn-test-connection"),
  btnSave: document.getElementById("btn-save-settings"),
  btnResetWs: document.getElementById("btn-reset-ws"),
  saveStatusMsg: document.getElementById("save-status-msg"),

  workspaceDir: document.getElementById("cfg-workspace-dir"),
  vscodeMode: document.getElementById("cfg-vscode-mode"),
  splitPdf: document.getElementById("cfg-split-pdf"),
  compiler: document.getElementById("cfg-compiler"),
  cppStd: document.getElementById("cfg-cpp-std"),
  sanitizer: document.getElementById("cfg-sanitizer"),
  timeLimit: document.getElementById("cfg-time-limit"),
  memoryLimit: document.getElementById("cfg-memory-limit"),
  autoDetect: document.getElementById("cfg-auto-detect"),
  notifications: document.getElementById("cfg-notifications")
};

let currentSettings = {};

async function checkConnection() {
  try {
    const res = await fetch(`${API_BASE}/api/status`, { cache: "no-store" });
    if (res.ok) {
      const data = await res.json();
      elements.statusBadge.className = "status-badge online";
      const sdStatus = data.streamdeck_connected ? " • Stream Deck OK" : " • Stream Deck: odłączony";
      elements.statusText.textContent = `Online (Port 8080)${sdStatus}`;
      return true;
    }
  } catch (_) {}

  elements.statusBadge.className = "status-badge offline";
  elements.statusText.textContent = "Offline (AlgoDeck nie działa)";
  return false;
}

async function loadSettings() {
  const isOnline = await checkConnection();

  if (isOnline) {
    try {
      const res = await fetch(`${API_BASE}/api/settings`, { cache: "no-store" });
      if (res.ok) {
        currentSettings = await res.json();
        applySettingsToForm(currentSettings);
        // Zapisz też lokalnie w chrome.storage dla rozszerzenia
        if (typeof chrome !== "undefined" && chrome.storage && chrome.storage.local) {
          chrome.storage.local.set({ algodeck_settings: currentSettings });
        }
        return;
      }
    } catch (e) {
      console.warn("Błąd pobierania ustawień z backendu:", e);
    }
  }

  // Fallback: pobierz z chrome.storage.local
  if (typeof chrome !== "undefined" && chrome.storage && chrome.storage.local) {
    chrome.storage.local.get(["algodeck_settings", "auto_detect_downloads"], (data) => {
      const s = data.algodeck_settings || {};
      if (data.auto_detect_downloads !== undefined) {
        s.auto_detect_downloads = data.auto_detect_downloads;
      }
      currentSettings = s;
      applySettingsToForm(currentSettings);
    });
  }
}

function applySettingsToForm(s) {
  if (s.workspace_dir) elements.workspaceDir.value = s.workspace_dir;
  if (s.vscode_mode) elements.vscodeMode.value = s.vscode_mode;
  if (s.open_pdf_in_vscode !== undefined) elements.splitPdf.checked = Boolean(s.open_pdf_in_vscode);
  if (s.cxx_compiler) elements.compiler.value = s.cxx_compiler;
  if (s.cpp_standard) elements.cppStd.value = s.cpp_standard;
  if (s.sanitize_enabled !== undefined) elements.sanitizer.checked = Boolean(s.sanitize_enabled);
  if (s.default_time_limit) elements.timeLimit.value = s.default_time_limit;
  if (s.default_memory_limit) elements.memoryLimit.value = s.default_memory_limit;
  if (s.auto_detect_downloads !== undefined) elements.autoDetect.checked = Boolean(s.auto_detect_downloads);
  if (s.notifications !== undefined) elements.notifications.checked = Boolean(s.notifications);
}

function collectSettingsFromForm() {
  return {
    ...currentSettings,
    workspace_dir: elements.workspaceDir.value.trim(),
    vscode_mode: elements.vscodeMode.value,
    open_pdf_in_vscode: elements.splitPdf.checked,
    cxx_compiler: elements.compiler.value,
    cpp_standard: elements.cppStd.value,
    sanitize_enabled: elements.sanitizer.checked,
    default_time_limit: parseFloat(elements.timeLimit.value) || 1.0,
    default_memory_limit: parseInt(elements.memoryLimit.value, 10) || 128,
    auto_detect_downloads: elements.autoDetect.checked,
    notifications: elements.notifications.checked
  };
}

async function saveSettings() {
  const newSettings = collectSettingsFromForm();
  elements.btnSave.disabled = true;
  elements.saveStatusMsg.textContent = "Zapisywanie...";
  elements.saveStatusMsg.className = "save-status-msg";

  // Zapisz w chrome.storage.local
  if (typeof chrome !== "undefined" && chrome.storage && chrome.storage.local) {
    chrome.storage.local.set({
      algodeck_settings: newSettings,
      auto_detect_downloads: newSettings.auto_detect_downloads
    });
  }

  // Wyślij do backendu AlgoDeck
  try {
    const res = await fetch(`${API_BASE}/api/settings`, {
      method: "POST",
      headers: { "Content-Type": "application/json" },
      body: JSON.stringify(newSettings)
    });

    if (res.ok) {
      currentSettings = newSettings;
      elements.saveStatusMsg.textContent = "✓ Ustawienia zostały pomyślnie zapisane!";
      elements.saveStatusMsg.className = "save-status-msg success";
    } else {
      elements.saveStatusMsg.textContent = "Zapisano lokalnie (Błąd serwera: " + res.status + ")";
      elements.saveStatusMsg.className = "save-status-msg";
    }
  } catch (err) {
    elements.saveStatusMsg.textContent = "Zapisano lokalnie w przeglądarce (AlgoDeck offline)";
    elements.saveStatusMsg.className = "save-status-msg";
  } finally {
    elements.btnSave.disabled = false;
    setTimeout(() => {
      elements.saveStatusMsg.textContent = "";
    }, 4000);
  }
}

// Event Listeners
document.addEventListener("DOMContentLoaded", () => {
  loadSettings();

  elements.btnSave.addEventListener("click", saveSettings);

  elements.btnTestConn.addEventListener("click", async () => {
    elements.saveStatusMsg.textContent = "Weryfikacja połączenia...";
    elements.saveStatusMsg.className = "save-status-msg";
    const ok = await checkConnection();
    elements.saveStatusMsg.textContent = ok ? "✓ Połączono z daemonem AlgoDeck!" : "Brak połączenia z http://127.0.0.1:8080";
    elements.saveStatusMsg.className = ok ? "save-status-msg success" : "save-status-msg error";
    setTimeout(() => { elements.saveStatusMsg.textContent = ""; }, 3000);
  });

  elements.btnClose.addEventListener("click", () => {
    window.close();
  });

  elements.btnResetWs.addEventListener("click", () => {
    elements.workspaceDir.value = "/home/linux/AlgoDeckWorkspace";
  });
});
