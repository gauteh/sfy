import os
from datetime import datetime, timedelta
import logging
import click
import json
import yaml
from mergedeep import merge
import numpy as np
import xarray as xr
from sfy.hub import Hub

logger = logging.getLogger(__name__)

@click.group()
def collection():
    pass

@collection.command()
@click.argument('config', type=click.File())
def archive(config):
    """Create CF-compatible trajectory file based on yml configuation file

    Presently only works for Floatensteins.
    Three steps:

    1) Make a template yml file, with the option to filter drifters on name (e.g. "cirfa"):

    $ sfydata collection template cirfa2022.yml -f cirfa

    2) The yml file can be edited if desired, e.g. drifters may be removed,
       and global attribute values added

    3) Create the netCDF file from given (possibly edited) yml file:

    $ sfydata collection archive cirfa2022.yml

    This will produce a netCDF file cirfa2022.nc which is compliant with
    the netCDF CF-specification for trajectories
    https://cfconventions.org/Data/cf-conventions/cf-conventions-1.10/cf-conventions.html#trajectory-data
    """

    import trajan as ta

    logger.info(f'Reading configuration file: {config.name}')

    with open(config.name, 'r') as f:
        config = yaml.safe_load(f)

    if 'start_time' in config:
        overall_start_time = config['start_time']
    else:
        overall_start_time = datetime(1,1,1)
    if 'end_time' in config:
        overall_end_time = config['end_time']
    else:
        overall_end_time = datetime.now()

    assert end_time > start_time, "end time must be greater than start time"

    hub = Hub.from_env()
    # Select only the buoys that are listed in config file
    buoys = [b for b in hub.buoys() if b.dev in config['drifters']]

    dicts = {}  # to hold all messages

    for b in buoys:
        bname = b.dev
        bc = config['drifters'][bname]

        if 'start_time' in config and config['start_time'] is not None:
            start_time = config['start_time']
        elif 'start_time' in bc and bc['start_time'] is not None:
            start_time = bc['start_time']
        else:
            start_time = datetime(1,1,1)

        if 'end_time' in config and config['end_time'] is not None:
            end_time = config['end_time']
        elif 'end_time' in bc and bc['end_time'] is not None:
            end_time = bc['end_time']
        else:
            end_time = datetime.now()

        dicts[bname] = {}
        packages = b.fetch_packages_range(start=datetime(1, 1, 1), end=datetime.now())
        for p in packages:
            j = json.loads(p[2])
            for m in j['body']['messages']:
                if 'latitude' in m:
                    time = datetime.fromtimestamp(m['datetime_fix'])
                    if time >= start_time and time <= end_time:
                        dicts[bname][time] = \
                            {'lon': m['longitude'], 'lat': m['latitude']}
                    else:
                        print(f'Skipping time {time}')

    ds = ta.trajectory_dict_to_dataset(dicts, config['attributes'])
    comments = [config['drifters'][bname]['comment'] if 'comment' in config['drifters'][bname]
                else '' for bname in config['drifters']]
    comments = [c if c is not None else '' for c in comments]
    if sum([len(c) for c in comments]) > 0:
        ds = ds.assign(drifter_description = (['trajectory'], comments))

    compression = { 'zlib': True }
    encoding = {}

    for v in ds.variables:
        encoding[v] = compression

    ds.to_netcdf(f"{config['name']}.nc", encoding=encoding)

@collection.command()
@click.argument('config', type=click.File('w'))
@click.option('-f', '--filter',
              default=None,
              help='Filter on drifter names (case insensitive)',
              type=str,
              multiple=True)
@click.option('-u', '--userconfig',
              default=None,
              help='YML file with config items to add',
              type=str,
              multiple=True)

def template(config, filter, userconfig):
    """Create template yml file which can be manually edited before creating netCDF"""

    logger.info(f'Writing configuration file: {config.name}')

    # Template yaml. Can add more standard attributes here, to be filled in config file by user
    t = {
        'name': f'{config.name.split(".")[0]}',
        'drifters': {},
        'attributes': {
            'title': '',
            'summary': '',
            'history': 'Created with sfydata.py (https://github.com/gauteh/sfy)',
            'creator_name': '',
            'creator_email': '',
            'creator_url': '',
            'references': '',
            }
        }

    # Fetch list of drifters
    hub = Hub.from_env()
    buoys = hub.buoys()

    drifters = [b.dev for b in hub.buoys()]
    if filter != ():
        for filterstring in filter:
            drifters = [b for b in drifters if filterstring.lower() in b.lower()]

    overall_end_time = datetime(1, 1, 1)
    overall_start_time = datetime.now()
    for b in hub.buoys():
        bname = b.dev
        if bname not in drifters:
            continue

        end_time = datetime(1, 1, 1)
        start_time = datetime.now()
        packages = b.fetch_packages_range(start=end_time, end=start_time)
        for p in packages:
            j = json.loads(p[2])
            for m in j['body']['messages']:
                if 'latitude' in m:
                    time = datetime.fromtimestamp(m['datetime_fix'])
                    start_time  = np.minimum(start_time, time)
                    end_time = np.maximum(end_time, time)
        t['drifters'][bname] = {'start_time': start_time, 'end_time': end_time, 'comment': ''}

        overall_start_time  = np.minimum(overall_start_time, start_time)
        overall_end_time = np.maximum(overall_end_time, end_time)

    t['start_time'] = overall_start_time
    t['end_time'] = overall_end_time

    if userconfig != ():
        for uc in userconfig:
            logger.info(f'Adding user config from {uc}')
            with open(uc, 'r') as f:
                usf = yaml.safe_load(f)
                t = merge(t, usf)  # merge without deleting what is not overwritten

    yaml.Dumper.ignore_aliases = lambda *args : True
    yaml.dump(t, open(config.name, 'w'), sort_keys=False)
