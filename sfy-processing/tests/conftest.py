from click.testing import CliRunner
import pytest


def pytest_addoption(parser):
    parser.addoption('--plot',
                     action='store_true',
                     help='Show plots',
                     default=False)


@pytest.fixture
def plot(pytestconfig):
    return pytestconfig.getoption('plot')


@pytest.fixture
def runner():
    return CliRunner()

