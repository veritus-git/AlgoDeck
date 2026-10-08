CPP_TEMPLATE = """/*
 * Zadanie: __TITLE__ (__PROBLEM_ID__)
 * Limit czasu: __TIME_LIMIT__s | Limit pamięci: __MEMORY_LIMIT__MB
 * Wygenerowano automatycznie przez AlgoDeck
 */

#pragma GCC optimize("O3,unroll-loops")
#include <bits/stdc++.h>
using namespace std;

using ll = long long;
using ld = long double;
using pii = pair<int, int>;
using pll = pair<ll, ll>;
using vi = vector<int>;
using vll = vector<ll>;

#define pb push_back
#define eb emplace_back
#define all(x) (x).begin(), (x).end()
#define sz(x) static_cast<int>((x).size())

#ifdef DEBUG
template<typename A, typename B> ostream& operator<<(ostream &os, const pair<A, B> &p) { return os << '(' << p.first << ", " << p.second << ')'; }
template<typename T_container, typename T = typename enable_if<!is_same<T_container, string>::value, typename T_container::value_type>::type>
ostream& operator<<(ostream &os, const T_container &v) { os << '{'; string sep; for (const T &x : v) os << sep << x, sep = ", "; return os << '}'; }
void dbg_out() { cerr << endl; }
template<typename Head, typename... Tail> void dbg_out(Head H, Tail... T) { cerr << ' ' << H; dbg_out(T...); }
#define dbg(...) cerr << "(" << #__VA_ARGS__ << "):", dbg_out(__VA_ARGS__)
#else
#define dbg(...)
#endif

void solve() {
    // Wpisz rozwiązanie tutaj
}

int main() {
    ios_base::sync_with_stdio(false);
    cin.tie(NULL);

    int t = 1;
    // cin >> t; // Odkomentuj jeśli występuje wiele zestawów danych
    while (t--) {
        solve();
    }

    return 0;
}
"""

BRUTE_CPP_TEMPLATE = """/*
 * Brute force (wzorzec naiwny) dla zadania {title} ({problem_id})
 * Wykorzystywany do automatycznego stress-testingu.
 */
#include <bits/stdc++.h>
using namespace std;

int main() {{
    ios_base::sync_with_stdio(false);
    cin.tie(NULL);

    // Zaimplementuj proste, bezbłędne O(N^2) lub wykładnicze rozwiązanie
    return 0;
}}
"""

GEN_PY_TEMPLATE = """#!/usr/bin/env python3
import random
import sys

# Generator losowych testów dla zadania {title} ({problem_id})
# Używany przez AlgoDeck / Stream Deck do stress-testingu

def main():
    # Przykładowy generator - dopasuj do ograniczeń zadania
    n = random.randint(1, 100)
    print(n)
    arr = [random.randint(1, 1000) for _ in range(n)]
    print(*(arr))

if __name__ == "__main__":
    main()
"""

MAKEFILE_TEMPLATE = """# Makefile wygenerowany przez AlgoDeck dla {problem_id}
CXX = g++
CXXFLAGS = -O3 -std=c++20 -Wall -Wextra
DEBUG_FLAGS = -g -O0 -std=c++20 -fsanitize=address,undefined -DDEBUG -Wall -Wextra
SRC = {problem_id}.cpp
TARGET = {problem_id}
DEBUG_TARGET = {problem_id}_debug

all: $(TARGET)

$(TARGET): $(SRC)
	$(CXX) $(CXXFLAGS) -o $(TARGET) $(SRC)

debug: $(SRC)
	$(CXX) $(DEBUG_FLAGS) -o $(DEBUG_TARGET) $(SRC)

clean:
	rm -f $(TARGET) $(DEBUG_TARGET) *.out

.PHONY: all debug clean
"""

VSCODE_TASKS_TEMPLATE = """{{
    "version": "2.0.0",
    "tasks": [
        {{
            "type": "shell",
            "label": "AlgoDeck: Kompilacja (O3)",
            "command": "g++ -O3 -std=c++20 -Wall -Wextra {problem_id}.cpp -o {problem_id}",
            "group": {{
                "kind": "build",
                "isDefault": true
            }},
            "problemMatcher": ["$gcc"]
        }},
        {{
            "type": "shell",
            "label": "AlgoDeck: Kompilacja Debug (ASan)",
            "command": "g++ -g -O0 -std=c++20 -fsanitize=address,undefined -DDEBUG -Wall -Wextra {problem_id}.cpp -o {problem_id}_debug",
            "group": "build",
            "problemMatcher": ["$gcc"]
        }}
    ]
}}
"""

VSCODE_LAUNCH_TEMPLATE = """{{
    "version": "0.2.0",
    "configurations": [
        {{
            "name": "AlgoDeck: Debug z GDB ({problem_id})",
            "type": "cppdbg",
            "request": "launch",
            "program": "${{workspaceFolder}}/{problem_id}_debug",
            "args": [],
            "stopAtEntry": false,
            "cwd": "${{workspaceFolder}}",
            "environment": [],
            "externalConsole": false,
            "MIMode": "gdb",
            "setupCommands": [
                {{
                    "description": "Włącz ładne drukowanie gdb",
                    "text": "-enable-pretty-printing",
                    "ignoreFailures": true
                }}
            ],
            "preLaunchTask": "AlgoDeck: Kompilacja Debug (ASan)"
        }}
    ]
}}
"""
