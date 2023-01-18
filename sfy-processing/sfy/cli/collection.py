import os
from datetime import datetime, timedelta
import itertools
import logging
import click
import json
import yaml
from mergedeep import merge
import numpy as np
import xarray as xr
import pandas as pd
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
        overall_start_time = datetime(1, 1, 1)
    if 'end_time' in config:
        overall_end_time = config['end_time']
    else:
        overall_end_time = datetime.now()

    assert overall_end_time > overall_start_time, "end time must be greater than start time"

    hub = Hub.from_env()
    # Select only the buoys that are listed in config file
    buoys = [b for b in hub.buoys() if b.dev in config['drifters']]

    dicts = {}  # to hold all messages
    var_attrs = {}

    imu = []
    list_frequencies = None

    for b in buoys:
        bname = b.dev
        bc = config['drifters'][bname]

        if 'start_time' in config and config['start_time'] is not None:
            start_time = config['start_time']
        elif 'start_time' in bc and bc['start_time'] is not None:
            start_time = bc['start_time']
        else:
            start_time = datetime(1, 1, 1)

        if 'end_time' in config and config['end_time'] is not None:
            end_time = config['end_time']
        elif 'end_time' in bc and bc['end_time'] is not None:
            end_time = bc['end_time']
        else:
            end_time = datetime.now()

        dicts[bname] = {}
        packages = b.fetch_packages_range(start=start_time, end=end_time)

        for p in packages:
            j = json.loads(p[2])
            ty = j.get('type', None)
            if ty == 'gps':
                for m in j['body']['messages']:
                    if 'latitude' in m:
                        time = datetime.fromtimestamp(m['datetime_fix'])
                        if time >= start_time and time <= end_time:
                            dicts[bname][time] = \
                                {'lon': m['longitude'], 'lat': m['latitude']}
                        else:
                            logger.debug(f'Skipping time {time}')
            elif ty == 'imu':
                messages = j.get('body').get('messages')
                for m in messages:
                    if list_frequencies is None:
                        list_frequencies = np.array(m.get('list_frequencies'))

                    time = datetime.fromtimestamp(m['datetime_fix'])
                    if time >= start_time and time <= end_time:
                        Hs = m.get('Hs')
                        Tz = m.get('Tz')
                        Tc = m.get('Tc')
                        accel_energy = m.get('list_acceleration_energies')
                        elevation_energy = m.get('list_elevation_energies')
                        m0 = m.get('wave_spectral_moments')['m0']
                        m2 = m.get('wave_spectral_moments')['m2']
                        m4 = m.get('wave_spectral_moments')['m4']
                        is_valid = m.get('is_valid')

                        # processed versions (low-freq cutoff)
                        pHs = m.get('processed_Hs', np.nan)
                        pTz = m.get('processed_Tz', np.nan)
                        pTc = m.get('processed_Tc', np.nan)
                        pelevation_energy = m.get('processed_list_elevation_energies', None)
                        if pelevation_energy is not None:
                            if len(pelevation_energy) != len(list_frequencies):
                                logger.debug('skipping old format elevation energies')
                                continue
                            else:
                                pelevation_energy = np.array(pelevation_energy)
                        else:
                            pelevation_energy = np.full((len(list_frequencies),), np.nan)

                        wm = m.get('processed_wave_spectral_moments', None)
                        if wm is not None:
                            pm0 = wm.get('m0')
                            pm2 = wm.get('m2')
                            pm4 = wm.get('m4')
                        else:
                            pm0 = np.nan
                            pm2 = np.nan
                            pm4 = np.nan

                        pcutoff = m.get('low_frequency_index_cutoff')

                        imu.append([
                            bname,
                            time,
                            Hs,
                            Tz,
                            Tc,
                            m0,
                            m2,
                            m4,
                            is_valid,
                            np.array(accel_energy),
                            np.array(elevation_energy),
                            pHs,
                            pTz,
                            pTc,
                            pm0,
                            pm2,
                            pm4,
                            pcutoff,
                            np.array(pelevation_energy),
                        ])
                    else:
                        logger.debug(f'Skipping time {time}')

    for b, v in dicts.items():
        logger.info(f'GPS observartions for {b}: {len(v)}')

    ds = ta.trajectory_dict_to_dataset(dicts,
                                       variable_attributes=var_attrs,
                                       global_attributes=config['attributes'])
    comments = [
        config['drifters'][bname]['comment']
        if 'comment' in config['drifters'][bname] else ''
        for bname in config['drifters']
    ]
    comments = [c if c is not None else '' for c in comments]
    if sum([len(c) for c in comments]) > 0:
        ds = ds.assign(drifter_description=(['trajectory'], comments))

    # Add IMU data
    if len(imu) > 0:
        # replace buoy name with trajectory number
        trajs = ds['drifter_names'].values
        for im in imu:
            tn = np.argwhere(trajs == im[0])[0][0]
            im[0] = tn

        imu = [[obs, *m] for obs, m in enumerate(imu)]

        logger.info(f'IMU observartions: {len(imu)}')

        ids = pd.DataFrame(imu,
                           columns=[
                               'imu_obs', 'trajectory', 'imu_time', 'Hs', 'Tz',
                               'Tc', 'm0', 'm2', 'm4', 'is_valid',
                               'accel_energy_spectrum',
                               'elevation_energy_spectrum', 'pHs', 'pTz',
                               'pTc', 'pm0', 'pm2', 'pm4', 'pcutoff',
                               'pelevation_energy'
                           ])

        # If any of the measurements have pHs values, drop messages without.
        if np.isnan(ids['pHs']).any() and not np.isnan(ids['pHs']).all():
            logger.error("IMU observations both with and without processed Hs, dropping those without.")
            ids = ids.loc[~np.isnan(ids['pHs'])]

        ids = ids.set_index((['trajectory', 'imu_obs']))
        ids = ids.to_xarray()


        ids = ids.drop_vars(
            ['imu_obs',
             'trajectory'])  # these are dimensions without coordinate values.

        # Move all observartions for each trajectory to starting row
        maxN = 0
        for ti in range(len(ds.trajectory)):
            iv = ~np.isnan(ids['imu_time'][ti, :])
            N = np.count_nonzero(iv)
            maxN = max(N, maxN)
            logger.debug(f'Condensing {ti=}, observations: {N}..')
            assert N > 0, "no valid observartions"

            for var in ids.variables:
                # logger.debug(f'Condensing {var}..')
                if ids[var].dtype != np.object and var[
                        0] != 'p':  # skip processed vars
                    n = np.count_nonzero(~np.isnan(ids[var][ti, :]))
                    assert n == N, f"Unexpected number of observations in trajectory for {ti=}, {var}: {n} != {N}."

                ids[var][ti, :N] = ids[var][ti, iv]
                ids[var][ti, N:] = np.nan

                if ids[var].dtype != np.object and var[0] != 'p':
                    assert (~np.isnan(ids[var][ti, :N])).all(
                    ), "Variying number of valid observartions within same trajectory."

        logger.info(f'Condensing imu_obs to: {maxN}')
        ids = ids.isel(imu_obs=slice(0, maxN))

        a = ids['accel_energy_spectrum'].values
        e = ids['elevation_energy_spectrum'].values
        pe = ids['pelevation_energy'].values

        ids = ids.drop_vars([
            'accel_energy_spectrum', 'elevation_energy_spectrum',
            'pelevation_energy'
        ])

        ds['imu_time'] = ids['imu_time']
        ds['Hs'] = ids['Hs']
        ds['Tz'] = ids['Tz']
        ds['Tc'] = ids['Tc']
        ds['m0'] = ids['m0']
        ds['m2'] = ids['m2']
        ds['m4'] = ids['m4']
        ds['imu_is_valid'] = ids['is_valid']
        ds['pHs'] = ids['pHs']
        ds['pTz'] = ids['pTz']
        ds['pTc'] = ids['pTc']
        ds['pm0'] = ids['pm0']
        ds['pm2'] = ids['pm2']
        ds['pm4'] = ids['pm4']
        ds['pcutoff'] = ids['pcutoff']

        sh = a.shape

        def coerce_spectra(S):
            S = [[
                np.full((len(list_frequencies), ), np.nan)
                if not isinstance(aa, np.ndarray) else aa for aa in aa
            ] for aa in S]
            S = np.stack(list(itertools.chain.from_iterable(S)))
            S = S.reshape((*sh, len(list_frequencies)))
            Sv = xr.DataArray(data=S,
                              dims=['trajectory', 'imu_obs', 'frequencies'],
                              coords=dict(frequencies=list_frequencies))
            return Sv

        # accel.attrs['frequencies'] = list_frequencies

        ds['accel_energy_spectrum'] = coerce_spectra(a)
        ds['elevation_energy_spectrum'] = coerce_spectra(e)
        ds['processed_elevation_energy_spectrum'] = coerce_spectra(pe)

    print(ds)

    compression = {'zlib': True}
    encoding = {}

    for v in ds.variables:
        encoding[v] = compression

    ds.to_netcdf(f"{config['name']}.nc", encoding=encoding)


@collection.command()
@click.argument('config', type=click.File('w'))
@click.option('-f',
              '--filter',
              default=None,
              help='Filter on drifter names (case insensitive)',
              type=str,
              multiple=True)
@click.option('-u',
              '--userconfig',
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
            'history':
            'Created with sfydata.py (https://github.com/gauteh/sfy)',
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
            drifters = [
                b for b in drifters if filterstring.lower() in b.lower()
            ]

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
                    start_time = np.minimum(start_time, time)
                    end_time = np.maximum(end_time, time)
        t['drifters'][bname] = {
            'start_time': start_time,
            'end_time': end_time,
            'comment': ''
        }

        overall_start_time = np.minimum(overall_start_time, start_time)
        overall_end_time = np.maximum(overall_end_time, end_time)

    t['start_time'] = overall_start_time
    t['end_time'] = overall_end_time

    if userconfig != ():
        for uc in userconfig:
            logger.info(f'Adding user config from {uc}')
            with open(uc, 'r') as f:
                usf = yaml.safe_load(f)
                t = merge(
                    t, usf)  # merge without deleting what is not overwritten

    yaml.Dumper.ignore_aliases = lambda *args: True
    yaml.dump(t, open(config.name, 'w'), sort_keys=False)
