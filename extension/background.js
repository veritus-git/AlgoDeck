// AlgoDeck ⚡ Koło MAP - Background Service Worker
// Automatyczne wykrywanie pobranych plików PDF i proponowanie 1-kliknięciem utworzenia workspace'u

const API_BASE = "http://127.0.0.1:8080";
const pendingPdfMap = new Map();

chrome.downloads.onChanged.addListener((delta) => {
  if (delta.state && delta.state.current === 'complete') {
    chrome.downloads.search({ id: delta.id }, (items) => {
      if (!items || items.length === 0) return;
      const item = items[0];
      const filename = item.filename || "";
      if (filename.toLowerCase().endsWith('.pdf')) {
        console.log("[AlgoDeck] Wykryto pobranie pliku PDF:", filename);
        const baseName = filename.split('/').pop() || "zadanie.pdf";
        const notifId = `algodeck_pdf_${delta.id}_${Date.now()}`;
        pendingPdfMap.set(notifId, filename);

        // 1. Wyświetl natywne powiadomienie z przyciskiem 1-kliknięcia
        try {
          chrome.notifications.create(notifId, {
            type: "basic",
            iconUrl: "icons/icon128.png",
            title: "AlgoDeck ⚡ Wykryto zadanie PDF",
            message: `Pobrano ${baseName}. Utwórz workspace jednym kliknięciem!`,
            buttons: [
              { title: "⚡ Utwórz Workspace (1 Klik)" },
              { title: "Otwórz szczegóły" }
            ],
            requireInteraction: true
          });
        } catch (e) {
          console.warn("[AlgoDeck] Błąd tworzenia powiadomienia:", e);
        }

        // 2. Otwórz poręczne okno popupa z propozycją utworzenia jednym kliknięciem
        openAutoPdfWindow(filename);
      }
    });
  }
});

function openAutoPdfWindow(filename) {
  const targetUrl = chrome.runtime.getURL(`popup.html?auto_pdf=${encodeURIComponent(filename)}`);
  chrome.windows.create({
    url: targetUrl,
    type: "popup",
    width: 420,
    height: 520,
    focused: true
  }).catch(err => {
    console.warn("[AlgoDeck] Błąd otwierania okna dla PDF:", err);
  });
}

// Obsługa kliknięcia w przycisk powiadomienia (1-KLIK)
chrome.notifications.onButtonClicked.addListener(async (notifId, btnIdx) => {
  const filename = pendingPdfMap.get(notifId);
  if (!filename) return;

  if (btnIdx === 0) {
    // 1-KLIK: Tworzenie workspace natychmiast bez żadnego przeciągania
    try {
      const res = await fetch(`${API_BASE}/api/auto-import-pdf`, {
        method: "POST",
        headers: { "Content-Type": "application/json" },
        body: JSON.stringify({ file_path: filename, auto_create: true })
      });
      const data = await res.json();
      chrome.notifications.clear(notifId);
      if (res.ok && data.success) {
        chrome.notifications.create(`success_${Date.now()}`, {
          type: "basic",
          iconUrl: "icons/icon128.png",
          title: "AlgoDeck ⚡ Workspace Gotowy",
          message: `Utworzono zadanie ${(data.problem_id || "ZAD").toUpperCase()}! VS Code został otwarty.`
        });
      }
    } catch (err) {
      console.error("[AlgoDeck] Błąd szybkiego tworzenia zadania:", err);
    }
  } else {
    // Otwórz szczegóły w oknie
    chrome.notifications.clear(notifId);
    openAutoPdfWindow(filename);
  }
});

chrome.notifications.onClicked.addListener((notifId) => {
  const filename = pendingPdfMap.get(notifId);
  if (filename) {
    chrome.notifications.clear(notifId);
    openAutoPdfWindow(filename);
  }
});
