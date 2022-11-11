import os
from datetime import datetime, timedelta
import logging
import click
import json
import yaml
import numpy as np
import xarray as xr
import trajan as ta
from sfy.hub import Hub

logger = logging.getLogger(__name__)

@click.group()
def collection():
    pass

@collection.command()
@click.argument('config', type=click.File())
def archive(config):
    """Create CF-compatible trajectory file based on yml configuation file

    Presently only works for Floatensteins
    """

    logger.info(f'Reading configuration file: {config.name}')

    with open(config.name, 'r') as f:
        config = yaml.safe_load(f)

    hub = Hub.from_env()
    # Select only the buoys that are listed in config file
    buoys = [b for b in hub.buoys() if b.dev in config['drifters']]

    dicts = {}  # to hold all messages

    for b in buoys:
        dicts[b] = {}
        packages = b.fetch_packages_range(start=datetime(1, 1, 1), end=datetime.now())
        for p in packages:
            j = json.loads(p[2])
            for m in j['body']['messages']:
                if 'latitude' in m:
                    dicts[b][datetime.fromtimestamp(m['datetime_fix'])] = \
                        {'lon': m['longitude'], 'lat': m['latitude']}

    ds = ta.trajectory_dict_to_dataset(dicts, config['attributes'])
    ds.to_netcdf(f"{config['name']}.nc")

@collection.command()
@click.argument('config', type=click.File('w'))
@click.option('-f', '--filter',
              default=None,
              help='Filter on drifter names (case insensitive)',
              type=str,
              multiple=True)

def template(config, filter):
    """Create template yml file which can be manually edited before creating netCDF"""

    logger.info(f'Writing configuration file: {config.name}')

    # Template yaml. Can add more standard attributes here, to be filled in config file by user
    t = {
        'name': f'{config.name.split(".")[0]}',
        'attributes': {
            'title': '',
            'summary': '',
            'history': '',
            'creator_name': '',
            'creator_email': '',
            'creator_url': '',
            'references': '',
            }
        }

    # Fetch list of drifters
    hub = Hub.from_env()
    b0 = hub.buoys()[0]
    t['drifters'] = [b.dev for b in hub.buoys()]
    if filter != ():
        for filterstring in filter:
            t['drifters'] = [b for b in t['drifters'] if filterstring.lower() in b.lower()]

    # Unfortunately order is not preserved, as OrderedDict is not supported:
    # https://stackoverflow.com/questions/5121931/in-python-how-can-you-load-yaml-mappings-as-ordereddicts)
    yaml.dump(t, open(config.name, 'w'))
