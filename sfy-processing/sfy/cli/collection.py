import os
from datetime import datetime, timedelta
import logging
import click
import json
import yaml
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

    ds = trajectory_dict_to_dataset(dicts, config['attributes'])
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

# This method might be moved to trajectory_analysis package?
def trajectory_dict_to_dataset(trajectory_dict, attributes=None):
    """
    trajectory_dict shall have the following structure:
        {'buoy1_name': {
            time0: {'lon': lon0, 'lat': lat0},
            time1: {'lon': lon1, 'lat': lat1},
                ...
            timeN: {'lon': lonN, 'lat': latN}},
        {'buoy2_name': {
            ...
    """

    drifter_names = [td.dev for td in trajectory_dict]
    num_drifters = len(trajectory_dict)
    num_times = np.max([len(d) for dn, d in trajectory_dict.items()])
    # Allocate  arrays
    lon = np.empty((num_drifters, num_times))
    lon[:] = np.nan
    lat = np.empty((num_drifters, num_times))
    lat[:] = np.nan
    time = np.empty((num_drifters, num_times), dtype='datetime64[s]')
    time[:] = np.datetime64('nat')

    # Fill arrays with data from dictionaries
    for drifter_num, (drifter_name, drifter_dict) in enumerate(trajectory_dict.items()):
        t = slice(0, len(drifter_dict))
        lon[drifter_num, t] = np.array([di['lon'] for d, di in drifter_dict.items()])
        lat[drifter_num, t] = np.array([di['lat'] for d, di in drifter_dict.items()])
        time[drifter_num, t] = np.array(list(drifter_dict), dtype='datetime64[s]')

    # Remove empty attributes
    attributes = {a:v for a,v in attributes.items() if v != ''}

    # Create Xarray Dataset adhering to CF conventions h.4.1 for trajectory data
    ds = xr.Dataset(
        data_vars=dict(
            lon=(['trajectory', 'obs'], lon,
                {'standard_name': 'longitude', 'unit': 'degree_east'}),
            lat=(['trajectory', 'obs'], lat,
                {'standard_name': 'latitude', 'unit': 'degree_north'}),
            time=(['trajectory', 'obs'], time,
                {'standard_name': 'time'}),
            drifter_names=(['trajectory'], drifter_names,
                {'cf_role': 'trajectory_id', 'standard_name': 'platform_id'})
            ),
        attrs={'Conventions': 'CF-1.10',
               'featureType': 'trajectory',
               'geospatial_lat_min': np.nanmin(lat),
               'geospatial_lat_max': np.nanmax(lat),
               'geospatial_lon_min': np.nanmin(lon),
               'geospatial_lon_max': np.nanmax(lon),
               'time_coverage_start': str(np.nanmin(time)),
               'time_coverage_end': str(np.nanmax(time)),
                **attributes
               }
    )

    return ds
