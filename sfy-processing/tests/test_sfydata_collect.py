import pytest
import os
import xarray as xr
import pandas as pd
import matplotlib.pyplot as plt
from pathlib import Path
from sfy.cli.sfydata import sfy

from . import *

@needs_hub
def test_cirfa_template(tmpdir, runner):
    with runner.isolated_filesystem(temp_dir=tmpdir) as td:
        td = Path(td)
        r = runner.invoke(sfy, ['--log=debug', 'collection', 'template', '-f', 'drifter_10_waves', 'cirfa.yaml', ])
        assert r.exit_code == 0


@needs_hub
def test_cirfa_archive(tmpdir, runner):
    with runner.isolated_filesystem(temp_dir=tmpdir) as td:
        td = Path(td)
        r = runner.invoke(sfy, ['--log=debug', 'collection', 'template', '-f', 'drifter_10_waves', 'cirfa.yaml', ])
        assert r.exit_code == 0

        r = runner.invoke(sfy, ['--log=debug', 'collection', 'archive', 'cirfa.yaml', ])
        assert r.exit_code == 0
