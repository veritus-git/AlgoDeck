# AlgoDeck ⚡ (Koło MAP)
### Zautomatyzowane środowisko do algorytmiki i programowania dla Koła MAP, OIJ, OI oraz Szkopuła
#### Architektura 100% RUST — Niezależna pojedyncza binarka, bezpośrednia obsługa USB HID Stream Decka, natywny Desktop GUI

> **Cel:** Zminimalizowanie progu wejścia i skrócenie czasu od pobrania zadania ze Szkopuła / MAP do pisania kodu w Visual Studio Code. Zero Pythona, zero zależności zewnętrznych aplikacji typu StreamController czy oficjalnego oprogramowania Elgato, zero żmudnego kopiowania testów.

---

## 💻 Instalatory One-Liner (Dla Każdego Systemu)

Zainstaluj całe środowisko jednym poleceniem w terminalu:

### 🐧 Linux (Ubuntu, Debian, Fedora, Arch)
```bash
curl -fsSL https://raw.githubusercontent.com/veritus-git/AlgoDeck/main/bootstrap.sh | bash
```

### 🍎 macOS (Apple Silicon M1/M2/M3/M4 & Intel)
```bash
curl -fsSL https://raw.githubusercontent.com/veritus-git/AlgoDeck/main/install_mac.sh | bash
```

### 🪟 Windows (PowerShell 5.1 / 7+)
```powershell
irm https://raw.githubusercontent.com/veritus-git/AlgoDeck/main/install_win.ps1 | iex
```

---

## 🚀 Kluczowe Cechy Architektury 100% Rust

### 1. 🦀 Pojedyncza, Błyskawiczna Binarka `algodeck`
- Całkowite wyeliminowanie Pythona, uvicorna, środowisk wirtualnych `.venv` oraz ciężkich bibliotek.
- **Zużycie pamięci RAM:** spadek z ponad 600 MB do zaledwie **~2-3 MB**!
- **Czas startu:** poniżej **15 milisekund**.
- Jednolity interfejs CLI:
  - `algodeck` — uruchamia natywne, nowoczesne okno GUI (`eframe`/`egui`) z obsługą systemowego zasobnika (tray).
  - `algodeck daemon` — uruchamia bezgłowy proces daemona w tle (serwer Axum REST/WebSocket + bezpośredni sterownik USB HID).
  - `algodeck start` / `stop` / `status` — szybkie zarządzanie procesem w tle.
  - `algodeck test <kod_zadania>` — błyskawiczne uruchomienie i weryfikacja wszystkich testów bezpośrednio z poziomu konsoli.
  - `algodeck run <kod_zadania>` — interaktywne uruchomienie skompilowanego rozwiązania C++.
  - `algodeck native-host` — bezpośredni mostek Native Messaging dla przeglądarki Chrome, Chromium, Brave i Edge.

### 2. 🎮 Bezpośredni Sterownik USB HID dla Elgato Stream Deck
- **Koniec problemów ze StreamControllerem i oficjalną aplikacją Elgato:** AlgoDeck komunikuje się z fizycznym kontrolerem Stream Deck bezpośrednio przez magistralę USB HID w czystym Rust.
- **Zero migania profili i opóźnień:** Klawisze LCD i OLED są rysowane i odświeżane w milisekundach w natywnym formacie.
- **Wsparcie dla wszystkich modeli:** Stream Deck Original v1/v2, MK.2, Mini, XL, Plus.
- **Tryb wirtualny:** W przypadku braku podłączonego fizycznego urządzenia USB, AlgoDeck działa płynnie w trybie wirtualnym przez Web Deck / Desktop GUI.

### 3. 🎨 Dopracowany Interfejs Desktop GUI
- **Ekran Zachęty (Standby Hero View):**
  - Minimalistyczny branding AlgoDeck Koło MAP.
  - **Duży, 168-pikselowy okrągły przycisk START** w nasyconej szmaragdowej zieleni (`#05af69` / `#10d77d`) z poświatą Cyber Emerald Halo Glow.
  - Wizualny stan ładowania w trakcie uruchamiania procesów w tle.
- **Centrum Zarządzania Zadaniami:**
  - `⚡ Import (PDF + ZIP)`: Dedykowane obszary dropzone dla treści zadania oraz archiwum ZIP, automatyczne parsowanie kodu, tytułu i limitów.
  - `✍ Ręczny Workspace`: Formularz do błyskawicznego tworzenia szablonu z własnymi danymi wejściowymi i wyjściowymi.
  - `📁 Zadania`: Przejrzysta galeria zadań olimpijskich z licznikami testów, otwieraniem w VS Code oraz 1-klik aktywacją.
  - `⚙ Ustawienia`: Czyste, czytelne okno konfiguracji folderu roboczego, kompilatora C++ (C++17/20/23) oraz integracji.

### 4. 🧩 Rozszerzenie do Przeglądarki i Wykrywanie Pobrań
- Pełna odporność na restart komputera: pobrania z przeszłości są ignorowane, brak spamu okienkami po ponownym uruchomieniu przeglądarki.
- Singleton window lock zapobiegający dublowaniu okien.
- 1-klik utworzenie projektu po pobraniu zadania ze Szkopuła lub Koła MAP.

---

## 🎮 Domyślny Układ Klawiszy Stream Decka (15 Klawiszy)

| Pozycja | Klawisz | Symbol | Działanie |
|:---:|:---:|:---:|:---|
| **0, 1, 3, 4** | Testy | `●` | Wskaźniki statusu poszczególnych testów |
| **2** | Zadanie | `KOD` | Wyświetla aktywny identyfikator zadania (np. `CHW`) |
| **5** | CODE | `</>` | Otwiera zadanie w Visual Studio Code (z podziałem ekranu na kod i PDF) |
| **6** | TEST | `⚗` | Kompiluje z optymalizacjami i uruchamia wszystkie testy |
| **7** | RUN | `▶` | Uruchamia program interaktywnie |
| **8** | KILL | `⏹` | Zatrzymuje wiszący lub zbyt długo wykonujący się proces |
| **9** | MENU | `⊞` | Otwiera okno panelu AlgoDeck |
| **10** | PREV | `‹` | Przełącza na poprzednie zadanie olimpijskie |
| **11, 12, 13** | Testy | `●` | Kolejne wskaźniki testów |
| **14** | NEXT | `›` | Przełącza na następne zadanie olimpijskie |

---

## 🛠 Wymagania i Kompilacja ze Źródeł

Wymagany jest standardowy kompilator C++ (`g++` lub `clang++`) oraz kompilator Rust (`cargo`):

```bash
git clone https://github.com/veritus-git/AlgoDeck.git
cd AlgoDeck
cargo build --release
cp target/release/algodeck ~/.local/bin/algodeck
```
