// AlgoDeck ⚡ Koło MAP - Background Service Worker
// Automatyczne wykrywanie pobranych plików PDF i otwieranie okna importu zadania

chrome.downloads.onChanged.addListener((delta) => {
  if (delta.state && delta.state.current === 'complete') {
    chrome.downloads.search({ id: delta.id }, (items) => {
      if (!items || items.length === 0) return;
      const item = items[0];
      const filename = item.filename || "";
      if (filename.toLowerCase().endsWith('.pdf')) {
        console.log("[AlgoDeck] Wykryto pobranie pliku PDF:", filename);
        
        // Otwórz modalne / popupowe okno AlgoDeck z przekazaną ścieżką do pobranego PDF
        const targetUrl = chrome.runtime.getURL(`popup.html?auto_pdf=${encodeURIComponent(filename)}`);
        
        chrome.windows.create({
          url: targetUrl,
          type: "popup",
          width: 480,
          height: 680,
          focused: true
        }).catch(err => {
          console.warn("[AlgoDeck] Błąd otwierania okna dla PDF:", err);
        });
      }
    });
  }
});
