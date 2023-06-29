import pytest
import os
from sfy import hub

def has_hub():
    API = os.getenv('SFY_SERVER')
    KEY = os.getenv('SFY_READ_TOKEN')

    if API is None or KEY is None:
        return False

    return True

needs_hub = pytest.mark.skipif(not has_hub(), reason="No data hub must configured.")

@pytest.fixture
def sfyhub(tmpdir):
    h = hub.Hub.from_env()
    h.cache = tmpdir
    return h
