# AlgoDeck ⚡ (Koło MAP)
### Zautomatyzowane środowisko do algorytmiki i programowania ze wsparciem Stream Decka i rozszerzenia w przeglądarce

> **Cel:** Zminimalizowanie progu wejścia i maksymalne skrócenie czasu od pobrania zadania do pisania kodu w Visual Studio Code. Zero żmudnego kopiowania testów, zero ręcznego tworzenia plików `.cpp` czy ręcznej konfiguracji skrótów.

---

## 🚀 Kluczowe Nowości & Architektura v2.0

1. **W 100% Lokalny Parser Olimpijski (Zero AI):**
   - Całkowicie usunięto zależność od modeli AI i kluczy API.
   - Błyskawiczny, deterministyczny parser dostosowany do arkuszy zadań z **Koła MAP**, **OIJ**, **OI** oraz **Szkopuła**.
   - Obsługa plików **PDF** oraz archiwów **ZIP** / folderów z testami:
     - Struktura z podfolderami `in/` i `out/` (np. `chw0a.in`, `chw0a.out` jak w zadaniu *Chwasty*).
     - Struktura płaska z samymi plikami wejściowymi `.in` (np. `akc0a.in` .. `akc3f.in` jak w zadaniu *Akcje*). Wzorzec dla testów przykładowych jest automatycznie wyciągany z treści PDF!

2. **Dedykowane Rozszerzenie w Przeglądarce (Chrome / Brave / Firefox):**
   - Koniec z koniecznością wchodzenia na osobną stronę www!
   - Rozszerzenie otwiera się bezpośrednio z paska przeglądarki jako okienko popup:
     - **Import (PDF + ZIP):** Przeciągnij plik PDF zadania i/lub paczkę testów ZIP. System w ułamku sekundy ekstrahuje limity, tworzy workspace olimpijski C++20, ładuje testy, profil Stream Decka i odpala VS Code.
     - **Ręczny Workspace:** Szybkie tworzenie zadania w 2 sekundy (wymagana tylko nazwa, np. `drzewo`, z opcjonalnym wklejeniem własnego testu wejścia/wyjścia).
     - **Galeria / Zadania:** Błyskawiczny podgląd wszystkich zadań, 1-click przełączenie aktywnego zadania na Stream Decku oraz otwarcie w VS Code.
     - **Wbudowany wirtualny Stream Deck:** Przyciski akcji na dole okienka z informacją zwrotną w czasie rzeczywistym.

3. **Minimalistyczny Profil Stream Decka (15 Klawiszy):**
   - Przejście na czyste, nowoczesne symbole bez zbędnych nakładek tekstowych:
     - `[ 2x0 ]` **BADGE ZADANIA:** Czysty wyświetlacz OLED z kodem zadania (np. `● AKC`).
     - `[ 0x1 ]` **VS CODE (`< / >`):** Natychmiastowe otwarcie/maksymalizacja okna VS Code z kodem zadania.
     - `[ 1x1 ]` **TESTUJ (`⚗`):** Uruchomienie kolorowego okna `test.sh` z porównaniem diff i pomiarami czasu.
     - `[ 2x1 ]` **ODPAL (`▶`):** Kompilacja i uruchomienie w zintegrowanym terminalu VS Code (skrót Ctrl+Shift+B).
     - `[ 3x1 ]` **KILL (`⏹`):** Natychmiastowe zatrzymanie wiszących procesów i pętli nieskończonych.
     - `[ 4x1 ]` **GALERIA (`⊞`):** Otwarcie podmenu ze wszystkimi zapisanymi zadaniami.
     - `[ 0x2 / 4x2 ]` **NAWIGACJA (`‹` i `›`):** Szybkie przełączanie między zadaniami.

4. **Kompletny Instalator One-Liner (100% One-Click):**
   - Wklejasz jedno polecenie do terminala — wszystko, czego brakowało w systemie, instaluje się automatycznie.

---

## 💻 Instalacja One-Click (Bootstrap)

Wystarczy wkleić jedno polecenie w terminalu:

```bash
curl -fsSL https://raw.githubusercontent.com/veritus-git/AlgoDeck/main/bootstrap.sh | bash
```

Lub jeśli masz sklonowane repozytorium:
```bash
./bootstrap.sh
```

### Co automatycznie wykonuje skrypt instalacyjny:
1. **Instaluje kompilatory C++ i narzędzia:** `gcc`, `g++`, `gdb`, `clang`, `make`, `cmake`, `valgrind`, `wl-clipboard`, `xclip`, `wmctrl`, `xdotool`, `gnome-terminal`, `unzip`.
2. **Konfiguruje uprawnienia Elgato Stream Deck:** Reguły udev USB (`/etc/udev/rules.d/60-streamdeck.rules`).
3. **Instaluje Visual Studio Code:** Oficjalne repozytorium Microsoftu + rozszerzenie C/C++ (`ms-vscode.cpptools`).
4. **Instaluje StreamController:** Oprogramowanie do obsługi Stream Decka pod Linuksem z Flathuba.
5. **Konfiguruje środowisko Pythona i ikony:** Czyste środowisko w `~/.local/share/algodeck/venv` i generowanie minimalistycznych symboli 144×144.
6. **Rejestruje usługę w systemd:** AlgoDeck uruchamia się jako cichy daemon użytkownika (`systemctl --user enable --now algodeck.service`).
7. **Przygotowuje rozszerzenie przeglądarki:** Gotowe do załadowania w Chrome/Brave lub uruchomienia poleceniem `algodeck-browser`.

---

## 🧩 Korzystanie z Rozszerzenia w Przeglądarce

### Jak załadować rozszerzenie do Google Chrome / Brave:
1. Otwórz w przeglądarce adres: `chrome://extensions` (lub `brave://extensions`).
2. Włącz suwak **Tryb programisty** (Developer mode) w prawym górnym rogu.
3. Kliknij przycisk **Załaduj rozpakowane** (Load unpacked) w lewym górnym rogu.
4. Wskaż katalog:
   ```
   ~/.local/share/algodeck/extension
   ```
   *(lub podfolder `extension/` w sklonowanym repozytorium)*.
5. Przypnij ikonę AlgoDeck ⚡ do paska narzędzi. Gotowe!

*Wskazówka:* Możesz także w każdej chwili wpisać w terminalu `algodeck-browser`, aby otworzyć przeglądarkę z wczytanym rozszerzeniem.

---

## 📁 Struktura Wygenerowanego Workspace Zadania

Po zaimportowaniu zadania (np. `akcje`), w `~/algodeck-workspace/akc/` tworzy się czysta struktura:

```
~/algodeck-workspace/akc/
├── akc.cpp               # JEDYNY WIDOCZNY PLIK dla użytkownika (szablon olimpijski C++20)
└── .algo/                # Ukryty katalog techniczny
    ├── statement.pdf     # Oryginalna treść zadania
    ├── problem.json      # Metadane i spis testów
    ├── test.sh           # Kolorowy test runner z pomiarami ms i diffem
    ├── run.sh            # Skrypt odpalania w terminalu VS Code (Ctrl+Shift+B)
    ├── kill.sh           # Bezpieczny SIGKILL zatrzymujący tylko to zadanie
    ├── accept.sh         # Skrypt zatwierdzania własnego wyniku jako wzorca
    └── tests/
        ├── akc0a.in / akc0a.out   # Oficjalny przykład z treści
        ├── akc1a.in               # Testy wydajnościowe z paczki ZIP / folderu
        └── ...
```

---

## 🎮 Klawisze Stream Decka

| Pozycja | Symbol | Działanie |
| :--- | :---: | :--- |
| **Góra (Środek)** | `● AKC` | Dynamiczny badge aktywnego zadania |
| **Środek 1** | `< / >` | Otwiera i maksymalizuje projekt w VS Code |
| **Środek 2** | `⚗` | Uruchamia okno testów ze wszystkimi przypadkami |
| **Środek 3** | `▶` | Odpala program w terminalu VS Code (oczekuje na cin) |
| **Środek 4** | `⏹` | Natychmiast ubija wiszący proces programu / testów |
| **Środek 5** | `⊞` | Otwiera galerię zadań na Stream Decku |
| **Dół Lewo / Prawo** | `‹` oraz `›` | Przełącza na poprzednie / następne zadanie |
