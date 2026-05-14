"""File contains utility functions for project."""
import uuid
from datetime import datetime, timezone


def job_id(timestamp: str|None=None) -> str:
    """Generate a unique job ID based on the current timestamp."""
    if timestamp is None:
        timestamp = time_now()
    return f"{timestamp}_{uuid.uuid4().hex[:8]}"

def time_now() -> str:
    """Generate a unique timestamp."""
    return datetime.now(timezone.utc).strftime("%Y%m%dT%H%M%SZ")
