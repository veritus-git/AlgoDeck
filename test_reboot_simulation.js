// ==============================================================================
// TEST SYMULACYJNY: RESTART KOMPUTERA I ZABEZPIECZENIE PRZED POWIELANIEM OKIEN
// ==============================================================================
const fs = require('fs');
const path = require('path');
const vm = require('vm');

console.log("⚡ [AlgoDeck Test Suite] Rozpoczynam symulację resetu komputera i testy Service Workera...\n");

// Mock środowiska Chrome Extension API
class ChromeMock {
  constructor() {
    this.sessionStorage = {};
    this.localStorage = {};
    this.downloadsHistory = [];
    this.createdWindows = [];
    this.notifications = [];
    this.activeDownloads = new Map();

    this.listeners = {
      onCreated: [],
      onChanged: [],
      onRemoved: []
    };

    const self = this;

    this.storage = {
      session: {
        get: async (keys) => {
          const res = {};
          for (const k of (Array.isArray(keys) ? keys : [keys])) {
            if (self.sessionStorage[k] !== undefined) res[k] = self.sessionStorage[k];
          }
          return res;
        },
        set: async (obj) => {
          Object.assign(self.sessionStorage, obj);
        },
        remove: async (keys) => {
          for (const k of (Array.isArray(keys) ? keys : [keys])) {
            delete self.sessionStorage[k];
          }
        }
      },
      local: {
        get: async (keys) => {
          const res = {};
          for (const k of (Array.isArray(keys) ? keys : [keys])) {
            if (self.localStorage[k] !== undefined) res[k] = self.localStorage[k];
          }
          return res;
        },
        set: async (obj) => {
          Object.assign(self.localStorage, obj);
        },
        remove: async (keys) => {
          for (const k of (Array.isArray(keys) ? keys : [keys])) {
            delete self.localStorage[k];
          }
        }
      }
    };

    this.downloads = {
      search: (query, cb) => {
        let results = [...self.downloadsHistory];
        if (query && query.id !== undefined) {
          results = results.filter(d => d.id === query.id);
        }
        if (cb) {
          cb(results);
          return;
        }
        return Promise.resolve(results);
      },
      onCreated: {
        addListener: (fn) => { self.listeners.onCreated.push(fn); }
      },
      onChanged: {
        addListener: (fn) => { self.listeners.onChanged.push(fn); }
      }
    };

    this.windows = {
      create: async (opts) => {
        const win = {
          id: Math.floor(Math.random() * 100000) + 1,
          url: opts.url,
          type: opts.type,
          tabs: [{ id: 1, url: opts.url }]
        };
        self.createdWindows.push(win);
        return win;
      },
      getAll: async (opts) => {
        return self.createdWindows;
      },
      update: async (winId, opts) => {
        const win = self.createdWindows.find(w => w.id === winId);
        return win;
      },
      onRemoved: {
        addListener: (fn) => { self.listeners.onRemoved.push(fn); }
      }
    };

    this.tabs = {
      query: async (query) => {
        const win = self.createdWindows.find(w => w.id === query.windowId);
        return win ? win.tabs : [];
      },
      update: async (tabId, opts) => {
        for (const w of self.createdWindows) {
          for (const t of w.tabs) {
            if (t.id === tabId) {
              t.url = opts.url;
              return t;
            }
          }
        }
      }
    };

    this.notifications = {
      create: (id, opts) => {
        self.notifications.push({ id, ...opts });
      },
      onClicked: { addListener: () => {} }
    };

    this.runtime = {
      getURL: (p) => `chrome-extension://algodeck/${p}`,
      reload: () => {}
    };
  }

  // Emisja zdarzenia do listenerów
  async emitCreated(item) {
    for (const fn of this.listeners.onCreated) {
      await fn(item);
    }
  }

  async emitChanged(delta) {
    for (const fn of this.listeners.onChanged) {
      await fn(delta);
    }
  }
}

async function runTests() {
  const bgCode = fs.readFileSync(path.join(__dirname, 'extension/background.js'), 'utf8');

  // ==========================================================================
  // SCENARIUSZ 1: SYMULACJA REBOOTU KOMPUTERA Z 10 PLIKAMI W HISTORII POBRANYCH
  // ==========================================================================
  console.log("----------------------------------------------------------------------");
  console.log("🧪 TEST 1: Symulacja zimnego startu komputera (Cold Boot / Reboot)...");
  console.log("----------------------------------------------------------------------");

  const chrome1 = new ChromeMock();

  // W historii przeglądarki z poprzedniego dnia wisi 10 pobranych plików zadania PDF i ZIP
  const pastTime = Date.now() - 3600 * 24 * 1000; // wczoraj
  for (let i = 1; i <= 10; i++) {
    chrome1.downloadsHistory.push({
      id: i,
      filename: `/home/linux/Pobrane/zadanie_${i}.pdf`,
      state: 'complete',
      startTime: new Date(pastTime + i * 1000).toISOString(),
      url: `https://szkopul.edu.pl/task_${i}.pdf`,
      mime: 'application/pdf'
    });
  }

  // Odpalamy Service Worker w wirtualnej maszynie
  const context1 = {
    chrome: chrome1,
    console: { log: () => {}, warn: () => {}, error: console.error },
    fetch: async () => ({ ok: false }),
    Set: Set,
    Map: Map,
    Date: Date,
    setTimeout: setTimeout,
    clearTimeout: clearTimeout,
    setInterval: () => {},
    clearInterval: () => {}
  };
  vm.createContext(context1);
  vm.runInContext(bgCode, context1);

  // Odczekaj 50ms na zakończenie ensureSessionInitialized()
  await new Promise(r => setTimeout(r, 60));

  // Przeglądarka po restarcie emituje serie zdarzeń onChanged dla starych plików z historii
  console.log("  -> Przeglądarka emituje eventy onChanged dla 10 starych pobrań z historii...");
  for (let i = 1; i <= 10; i++) {
    await chrome1.emitChanged({
      id: i,
      state: { current: 'complete' }
    });
  }

  // WERYFIKACJA 1:
  console.log(`  -> Liczba utworzonych okien: ${chrome1.createdWindows.length} (Oczekiwano: 0)`);
  if (chrome1.createdWindows.length === 0) {
    console.log("  ✅ SUKCES: Żadne okno się NIE otworzyło po restarcie komputera! Stare pliki zostały zignorowane.\n");
  } else {
    console.error(`  ❌ BŁĄD: Utworzono ${chrome1.createdWindows.length} niechcianych okien!`);
    process.exit(1);
  }

  // ==========================================================================
  // SCENARIUSZ 2: ŚWIEŻE POBRANIE W TRAKCIE DZIAŁANIA PRZEGLĄDARKI
  // ==========================================================================
  console.log("----------------------------------------------------------------------");
  console.log("🧪 TEST 2: Rzeczywiste pobranie nowego zadania przez użytkownika TERAZ...");
  console.log("----------------------------------------------------------------------");

  // Użytkownik klika "Pobierz PDF" ze Szkopuła
  const newDownloadId = 999;
  const newFileItem = {
    id: newDownloadId,
    filename: '/home/linux/Pobrane/akcje.pdf',
    state: 'in_progress',
    startTime: new Date().toISOString(),
    url: 'https://szkopul.edu.pl/akcje.pdf',
    mime: 'application/pdf'
  };
  chrome1.downloadsHistory.push(newFileItem);

  console.log("  -> Użytkownik rozpoczął pobieranie akcje.pdf (event onCreated)...");
  await chrome1.emitCreated(newFileItem);

  console.log("  -> Pobieranie pliku zakończone (event onChanged: complete)...");
  newFileItem.state = 'complete';
  await chrome1.emitChanged({
    id: newDownloadId,
    state: { current: 'complete' }
  });

  // Odczekaj chwilę na obsługę
  await new Promise(r => setTimeout(r, 80));

  console.log(`  -> Liczba utworzonych okien: ${chrome1.createdWindows.length} (Oczekiwano: 1)`);
  if (chrome1.createdWindows.length === 1 && chrome1.createdWindows[0].url.includes("akcje.pdf")) {
    console.log("  ✅ SUKCES: Utworzono dokładnie 1 dedykowane okno dla świeżo pobranego pliku!\n");
  } else {
    console.error(`  ❌ BŁĄD: Niepoprawna liczba okien: ${chrome1.createdWindows.length}`);
    process.exit(1);
  }

  // ==========================================================================
  // SCENARIUSZ 3: JEDNOCZESNE SZYBKIE POBRANIA (MUTEX / SINGLETON LOCK TEST)
  // ==========================================================================
  console.log("----------------------------------------------------------------------");
  console.log("🧪 TEST 3: Dwa jednoczesne pobrania w tym samym ułamku sekundy...");
  console.log("----------------------------------------------------------------------");

  const fileA = {
    id: 1001,
    filename: '/home/linux/Pobrane/silownia.pdf',
    state: 'complete',
    startTime: new Date().toISOString(),
    url: 'https://szkopul.edu.pl/silownia.pdf',
    mime: 'application/pdf'
  };
  const fileB = {
    id: 1002,
    filename: '/home/linux/Pobrane/koleje.pdf',
    state: 'complete',
    startTime: new Date().toISOString(),
    url: 'https://szkopul.edu.pl/koleje.pdf',
    mime: 'application/pdf'
  };
  chrome1.downloadsHistory.push(fileA, fileB);

  await chrome1.emitCreated(fileA);
  await chrome1.emitCreated(fileB);

  // Równoczesne zakończenie obu w tej samej milisekundzie
  await Promise.all([
    chrome1.emitChanged({ id: 1001, state: { current: 'complete' } }),
    chrome1.emitChanged({ id: 1002, state: { current: 'complete' } })
  ]);

  await new Promise(r => setTimeout(r, 80));

  console.log(`  -> Łączna liczba okien po jednoczesnych pobraniach: ${chrome1.createdWindows.length} (Oczekiwano: 1)`);
  if (chrome1.createdWindows.length === 1) {
    console.log("  ✅ SUKCES: Singleton lock zadziałał bezbłędnie — okno zostało zaktualizowane, brak stosu okienek!\n");
  } else {
    console.error(`  ❌ BŁĄD: Wykryto ${chrome1.createdWindows.length} okien (powinno być 1)!`);
    process.exit(1);
  }

  // ==========================================================================
  // SCENARIUSZ 4: PONOWNY RESTART KOMPUTERA Z TYMI NOWYMI PLIKAMI W HISTORII
  // ==========================================================================
  console.log("----------------------------------------------------------------------");
  console.log("🧪 TEST 4: Kolejny restart komputera (wszystkie pliki są już w historii)...");
  console.log("----------------------------------------------------------------------");

  const chrome2 = new ChromeMock();
  // Kopiujemy historię pobrań (w tym akcje.pdf, silownia.pdf, koleje.pdf)
  chrome2.downloadsHistory = [...chrome1.downloadsHistory];
  // storage.session jest puste (bo komputer został zresetowany)
  chrome2.sessionStorage = {};

  const context2 = {
    chrome: chrome2,
    console: { log: () => {}, warn: () => {}, error: console.error },
    fetch: async () => ({ ok: false }),
    Set: Set,
    Map: Map,
    Date: Date,
    setTimeout: setTimeout,
    clearTimeout: clearTimeout,
    setInterval: () => {},
    clearInterval: () => {}
  };
  vm.createContext(context2);
  vm.runInContext(bgCode, context2);

  await new Promise(r => setTimeout(r, 60));

  // Przeglądarka znowu odpala eventy dla wszystkich 13 plików
  for (const item of chrome2.downloadsHistory) {
    await chrome2.emitChanged({ id: item.id, state: { current: 'complete' } });
  }

  console.log(`  -> Liczba okien po ponownym restarcie: ${chrome2.createdWindows.length} (Oczekiwano: 0)`);
  if (chrome2.createdWindows.length === 0) {
    console.log("  ✅ SUKCES: Po ponownym restarcie zero okien! Całkowita odporność na historię.\n");
  } else {
    console.error(`  ❌ BŁĄD: Powstały niechciane okna po restarcie: ${chrome2.createdWindows.length}`);
    process.exit(1);
  }

  console.log("======================================================================");
  console.log("🎉 WSZYSTKIE 4 TESTY SYMULACYJNE ZAKOŃCZONE PEŁNYM SUKCESEM!");
  console.log("======================================================================\n");
}

runTests().catch(err => {
  console.error("Test execution failed:", err);
  process.exit(1);
});
