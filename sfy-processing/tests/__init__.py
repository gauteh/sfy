import pytest
from sfy import hub


@pytest.fixture
def sfyhub(tmpdir):
    h = hub.Hub.from_env()
    h.cache = tmpdir
    return h
