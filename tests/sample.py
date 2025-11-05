#!/usr/bin/env python3
"""Sample Python file for testing runtime tracing."""

def add_numbers(a, b):
    """Add two numbers together."""
    return a + b

from typing import Any, Iterable

def process_list(items: Iterable[Any]) -> list[Any]:
    """Process a list of items."""
    result = []
    for item in items:
        if isinstance(item, bool):
            result.append(item)  # keep booleans as-is
        elif isinstance(item, int):
            result.append(item * 2)
        elif isinstance(item, str):
            result.append(item.upper())
    return result

class Calculator:
    """A simple calculator class."""

    def __init__(self, initial_value=0):
        self.value = initial_value

    def add(self, x):
        self.value += x
        return self.value

    def multiply(self, x):
        self.value *= x
        return self.value

def test_add_numbers():
    """Test the add_numbers function."""
    assert add_numbers(2, 3) == 5
    assert add_numbers(-1, 1) == 0
    assert add_numbers(0.5, 1.5) == 2.0

def test_process_list():
    """Test the process_list function."""
    result = process_list([1, "hello", 2, "world"])
    expected = [2, "HELLO", 4, "WORLD"]
    assert result == expected

def test_calculator():
    """Test the Calculator class."""
    calc = Calculator(10)
    assert calc.add(5) == 15
    assert calc.multiply(2) == 30

if __name__ == "__main__":
    test_add_numbers()
    test_process_list()
    test_calculator()
    print("All tests passed!")
