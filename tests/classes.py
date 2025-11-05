#!/usr/bin/env python3
"""Test file for tracing class methods and more complex interactions."""

class MathOperations:
    """A class with various mathematical operations."""

    def __init__(self, initial_value: float = 0.0):
        self.value = initial_value
        self.history = []

    def add(self, x: float) -> float:
        """Add a number to the current value."""
        self.value += x
        self.history.append(('add', x))
        return self.value

    def multiply(self, x: float) -> float:
        """Multiply the current value by a number."""
        self.value *= x
        self.history.append(('multiply', x))
        return self.value

    def get_history(self) -> list:
        """Get the operation history."""
        return self.history.copy()

def process_numbers(numbers: list) -> dict:
    """Process a list of numbers and return statistics."""
    if not numbers:
        return {"count": 0, "sum": 0, "average": 0.0}
    total = sum(numbers)
    count = len(numbers)
    average = total / count

    return {
        "count": count,
        "sum": total,
        "average": average,
        "min": min(numbers),
        "max": max(numbers)
    }

def test_math_operations():
    """Test the MathOperations class."""
    math_ops = MathOperations(10.0)

    result1 = math_ops.add(5.0)
    assert result1 == 15.0

    result2 = math_ops.multiply(2.0)
    assert result2 == 30.0

    history = math_ops.get_history()
    assert len(history) == 2
    assert history[0] == ('add', 5.0)
    assert history[1] == ('multiply', 2.0)

def test_process_numbers():
    """Test the process_numbers function."""
    numbers = [1, 2, 3, 4, 5]
    result = process_numbers(numbers)

    assert result["count"] == 5
    assert result["sum"] == 15
    assert result["average"] == 3.0
    assert result["min"] == 1
    assert result["max"] == 5

    # Test empty list
    empty_result = process_numbers([])
    assert empty_result["count"] == 0

if __name__ == "__main__":
    test_math_operations()
    test_process_numbers()
    print("All class method tests passed!")
