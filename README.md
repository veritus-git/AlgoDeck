# AlgoDeck ⚡ (Koło MAP)
### Zautomatyzowane środowisko do algorytmiki i programowania dla Koła MAP, OIJ, OI oraz Szkopuła
#### Obsługa Linux, macOS (Apple Silicon & Intel) oraz Windows ze wsparciem oficjalnego oprogramowania Stream Deck i natywnym GUI w Rust

> **Cel:** Zminimalizowanie progu wejścia i maksymalne skrócenie czasu od pobrania zadania ze Szkopuła / MAP do pisania kodu w Visual Studio Code. Zero żmudnego kopiowania testów, zero ręcznego tworzenia plików `.cpp` czy ręcznej konfiguracji skrótów.

---

## 💻 Instalatory One-Liner (Dla Każdego Systemu)

Zainstaluj całe środowisko jednym poleceniem skopiowanym do terminala:

### 🍎 macOS (Apple Silicon M1/M2/M3/M4 & Intel — Priorytet)
Otwórz Terminal i wklej:
```bash
curl -fsSL https://raw.githubusercontent.com/veritus-git/AlgoDeck/main/install_mac.sh | bash
```

### 🐧 Linux (Ubuntu, Debian, Fedora, Arch)
Otwórz Terminal i wklej:
```bash
curl -fsSL https://raw.githubusercontent.com/veritus-git/AlgoDeck/main/bootstrap.sh | bash
```

### 🪟 Windows (PowerShell 5.1 / 7+)
Otwórz PowerShell i wklej:
```powershell
irm https://raw.githubusercontent.com/veritus-git/AlgoDeck/main/install_win.ps1 | iex
```

---

## 🚀 Kluczowe Nowości & Architektura v2.1

### 1. 🦀 Nowoczesny, Błyskawiczny Panel Kontrolny w RUST (`algodeck-gui`)
- Całkowite odejście od starego, ociężałego menu na rzecz natywnego interfejsu w **Rust** (`eframe` / `egui`).
- **Autorski Customowy Pasek Tytułowy:** Płynne przesuwanie okna (dragging), minimalizacja i zamykanie. Zgodnie z założeniami brak zbędnego rozszerzania na pełny ekran.
- **Stylistyka Cyberpunk / Deep Obsidian:** Czysta ciemna paleta `#080b11`, akcenty neonowego błękitu `#38bdf8` i szmaragdu `#10b981`.
- **Trzy Główne Sekcje:**
  - `➕ Nowe Zadanie`: Przełącznik między automatycznym importem PDF/ZIP a ręcznym wpisaniem (cin/cout).
  - `📋 Zadania`: Przegląd zadań z badge'ami `⚡ [AKTYWNE]`, liczba testów, 1-klik aktywacja na Stream Decku, otwieranie w VS Code, usuwanie zadań.
  - `⚙️ Ustawienia & Zaawansowane Tweakowanie`:
    - Ścieżka katalogu roboczego (`algodeck-workspace`).
    - Standard C++: `C++17`, `C++20` (domyślny), `C++23`.
    - Tryb Strict Debug: flaga `-fsanitize=address,undefined` do natychmiastowego wykrywania wycieków pamięci i UB.
    - Zachowanie VS Code (pojedyncze okno z podfolderami vs osobne okna).
    - Platforma docelowa Stream Decka (Linux StreamController / macOS Elgato / Windows Elgato) i numer seryjny.
    - Przyciski szybkiej synchronizacji profili oraz powrotu do ekranu zachęty (`ALGO_IDLE`).
- **Wieloplatformowa Kompilacja:** GitHub Actions automatycznie kompiluje binarki na Linux, macOS (Universal arm64/x86_64) i Windows!

### 2. 🎮 Pełne Wsparcie macOS i Windows (Oficjalna Aplikacja Elgato Stream Deck)
- **Linux:** Automatyczna synchronizacja stron JSON z oprogramowaniem **StreamController** (Flatpak).
- **macOS i Windows:** AlgoDeck dostarcza oficjalny plugin Elgato Stream Deck (`com.algodeck.streamdeck.sdPlugin`):
  - Instalowany bezpośrednio do katalogu wtyczek Elgato (`~/Library/Application Support/com.elgato.StreamDeck/Plugins` na macOS oraz `%APPDATA%\Elgato\StreamDeck\Plugins` na Windows).
  - Dwukierunkowa komunikacja WebSocket w czasie rzeczywistym z lokalnym serwerem AlgoDeck (`127.0.0.1:8080`).
  - Identyczne działanie na każdym systemie operacyjnym: dynamiczne wyświetlanie kodu zadania na klawiszu OLED, kompilacja, uruchamianie, testowanie i ubijanie wiszących procesów!

### 3. ⚡ Uruchamianie na Żądanie (On-Demand) i Nowy Klawisz Wyjścia
- **W Przeglądarce:** Po otwarciu rozszerzenia widoczny jest **DUŻY, ZIELONY OKRĄGŁY PRZYCISK "URUCHOM"**.
  - Kliknięcie uruchamia AlgoDeck i natychmiast aktywuje ekran zachęty (`ALGO_IDLE`) na Stream Decku z logiem i przyciskiem startowym.
- **W Terminalu:** Wpisanie polecenia `algodeck`:
  - Uruchamia środowisko w 100% w tle (jako odłączony proces / daemon).
  - Wyświetla natywne powiadomienie na pulpicie (`notify-send` na Linuksie, powiadomienie AppleScript na macOS, Windows Toast na Windowsie).
  - Terminal natychmiast wraca do znaku zachęty — zamknięcie terminala **nie zamyka** programu!
- **Zawinięta Strzałka Wyjścia na Stream Decku (`↩`):**
  - W ostatnim wolnym polu dolnego rzędu Stream Decka (`1x2`, zaraz obok ikony ustawień `0x2`) zaimplementowano nową, geometryczną zawiniętą strzałkę powrotu.
  - Kliknięcie jej natychmiast kończy pracę nad bieżącym zadaniem i przywraca czysty ekran zachęty (`ALGO_IDLE`).

### 4. 🧩 Niezawodne Wykrywanie Pobrań i Izolowany Widok 1-Klik
- **Naprawiono problem z pobraniami:** Wykrywanie działa w 100% deterministycznie na każdym systemie i dystrybucji dzięki wielopoziomowej weryfikacji stanu pobierania (retry backoff, sprawdzanie plików `.pdf` oraz `.zip` ze Szkopuła i MAP, systemowe notyfikacje OS Toast).
- **Izolowane Okno Powiadomienia:** Gdy pobranie zostanie wykryte, rozszerzenie wyświetla **WYŁĄCZNIE** elegancką kartę powiadomienia z opisem zadania, limitami oraz przyciskiem:
  > **⚡ Utwórz Workspace i Otwórz VS Code (1 Klik)**
  Cały standardowy układ popupa jest całkowicie ukryty, zapobiegając rozpraszaniu uwagi.

---

## 🎮 Układ Klawiszy Stream Decka (15 Klawiszy)

### Widok Zadania (np. `CHW` lub `AKC`):
| Pozycja | Klawisz | Symbol | Działanie |
| :--- | :---: | :---: | :--- |
| **Góra (0x0 .. 4x0)** | `0x0, 4x0` | `=` | Ozdobne separatory OLED |
| | `1x0 .. 3x0` | `● C ● H ● W` | Dynamiczne litery aktywnego zadania |
| **Środek (0x1 .. 4x1)** | `0x1` | `➕` | Dodaj nowe zadanie (otwiera okno w Rust) |
| | `1x1` | `⚗` | Uruchom test runner ze wszystkimi testami |
| | `2x1` | `▶` | Odpal w terminalu VS Code (oczekuje na cin) |
| | `3x1` | `⏹` | Natychmiast ubij wiszące pętle nieskończone |
| | `4x1` | `⊞` | Galeria wszystkich zapisanych zadań |
| **Dół (0x2 .. 4x2)** | `0x2` | `⚙` | Otwórz Centrum Kontroli i Ustawienia w Rust |
| | **`1x2`** | **`↩`** | **Zawinięta strzałka wyjścia — powrót do ekranu zachęty (ALGO_IDLE)** |
| | `2x2` | `‹` | Poprzednie zadanie w workspace |
| | `3x2` | `[ 1/3 ]` | Wskaźnik numeru zadania w kolejności chronologicznej |
| | `4x2` | `›` | Następne zadanie w workspace |

---

## 🔧 Komendy CLI

Po instalacji w terminalu dostępne są intuicyjne polecenia:

```bash
algodeck           # Uruchamia AlgoDeck w tle i aktywuje ekran zachęty na Stream Decku
algodeck status    # Sprawdza status daemona i aktywnego zadania
algodeck settings  # Otwiera natywne okno ustawień w Rust
algodeck stop      # Zatrzymuje działanie daemona w tle
```

---

## 🧩 Ładowanie Rozszerzenia w Przeglądarce

1. Otwórz w Google Chrome, Brave lub Edge adres: `chrome://extensions`.
2. Włącz suwak **Tryb programisty** (Developer mode) w prawym górnym rogu.
3. Kliknij **Załaduj rozpakowane** (Load unpacked) i wybierz katalog `extension/` z repozytorium (lub `~/.local/share/algodeck/extension`).
4. Przypnij ikonę AlgoDeck ⚡ do paska przeglądarki.
