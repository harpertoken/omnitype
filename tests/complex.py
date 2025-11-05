#!/usr/bin/env python3
"""More complex Python file for testing runtime tracing with various types."""

from typing import Dict, List, Optional, Union
import json

def process_data(data: Dict[str, Union[int, str, List[int]]]) -> Dict[str, str]:
    """Process complex nested data structures."""
    result = {}
    for key, value in data.items():
        if isinstance(value, int):
            result[key] = f"number: {value}"
        elif isinstance(value, str):
            result[key] = f"text: {value}"
        elif isinstance(value, list):
            result[key] = f"list with {len(value)} items"
    return result

def find_user(users: List[Dict[str, Union[str, int]]], user_id: int) -> Optional[Dict[str, Union[str, int]]]:
    """Find a user by ID in a list of user dictionaries."""
    for user in users:
        if user.get("id") == user_id:
            return user
    return None

class DataProcessor:
    """A class for processing various data types."""

    def __init__(self):
        self.processed_count = 0
        self.cache: Dict[str, Union[str, int, List]] = {}

    def process_json(self, json_str: str) -> Dict:
        """Parse and process JSON data."""
        try:
            data = json.loads(json_str)
            self.processed_count += 1
            return data
        except json.JSONDecodeError:
            return {}

    def cache_result(self, key: str, value: Union[str, int, List]) -> None:
        """Cache a processing result."""
        self.cache[key] = value

def test_process_data():
    """Test the process_data function."""
    test_data = {
        "name": "Alice",
        "age": 30,
        "scores": [85, 92, 78]
    }
    result = process_data(test_data)
    assert "name" in result
    assert "age" in result
    assert "scores" in result

def test_find_user():
    """Test the find_user function."""
    users = [
        {"id": 1, "name": "Alice", "age": 30},
        {"id": 2, "name": "Bob", "age": 25},
        {"id": 3, "name": "Charlie", "age": 35}
    ]

    user = find_user(users, 2)
    assert user is not None
    assert user["name"] == "Bob"

    missing_user = find_user(users, 999)
    assert missing_user is None

def test_data_processor():
    """Test the DataProcessor class."""
    processor = DataProcessor()

    json_data = '{"key": "value", "number": 42}'
    result = processor.process_json(json_data)
    assert result["key"] == "value"
    assert result["number"] == 42
    assert processor.processed_count == 1

    processor.cache_result("test", [1, 2, 3])
    assert "test" in processor.cache

if __name__ == "__main__":
    test_process_data()
    test_find_user()
    test_data_processor()
    print("All complex type tests passed!")
