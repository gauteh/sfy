import pytest

def pytest_addoption(parser):
    parser.addoption('--plot', action='store_true', help='Show plots', default=False)

@pytest.fixture
def plot(pytestconfig):
    return pytestconfig.getoption('plot')
