CPP_TEMPLATE = """#include <iostream>
using namespace std;

int main() {
    ios_base::sync_with_stdio(false);
    cin.tie(NULL);

}
"""

MAKEFILE_TEMPLATE = """CXX = g++
CXXFLAGS = -O3 -std=c++20 -Wall -Wextra
TARGET = {problem_id}

all: $(TARGET)

$(TARGET): {problem_id}.cpp
	$(CXX) $(CXXFLAGS) -o $(TARGET) {problem_id}.cpp

clean:
	rm -f $(TARGET) *.out

.PHONY: all clean
"""
