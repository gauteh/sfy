import pytest
import os
from dotenv import load_dotenv
from sfy import hub

def has_hub():
    load_dotenv()

    API = os.getenv('SFY_SERVER')
    KEY = os.getenv('SFY_READ_TOKEN')

    print(API, KEY)

    if API is None or KEY is None:
        return False

    return True

needs_hub = pytest.mark.skipif(not has_hub(), reason="A data hub must be configured.")

@pytest.fixture
def sfyhub(tmpdir):
    h = hub.Hub.from_env()
    h.cache = tmpdir
    return h
