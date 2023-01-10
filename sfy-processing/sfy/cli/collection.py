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
        overall_start_time = datetime(1,1,1)
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
            start_time = datetime(1,1,1)

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
                            print(f'Skipping time {time}')
            elif ty == 'imu':
                messages = j.get('body').get('messages')
                for m in messages:
                    if list_frequencies is None:
                        list_frequencies = np.array(m.get('list_frequencies'))

                    time = datetime.fromtimestamp(m['datetime_fix'])
                    Hs = m.get('Hs')
                    Tz = m.get('Tz')
                    Tc = m.get('Tc')
                    accel_energy = m.get('list_acceleration_energies')
                    m0 = m.get('wave_spectral_moments')['m0']
                    m2 = m.get('wave_spectral_moments')['m2']
                    m4 = m.get('wave_spectral_moments')['m4']
                    is_valid = m.get('is_valid')

                    imu.append([bname, time, Hs, Tz, Tc, m0, m2, m4, is_valid, np.array(accel_energy)])
                    # {'account': 'gauteh@met.no', 'datetime': 1673331884000.0, 'device': 'OO-2023-10-LM', 'type': 'imu', 'body': {'iridium_pos': {'lat': -34.95641666666667, 'lon': 24.487466666666666}, 'messages': [{'datetime_fix': 1673331709.0, 'spectrum_number': 62, 'Hs': 2.7734711170196533, 'Tz': 9.117854579102532, 'Tc': 7.282742646351041, '_array_max_value': 1.9652477502822876, '_array_uint16': [18, 18, 39, 142, 537, 2261, 6064, 12857, 18489, 47039, 65000, 39780, 13904, 36674, 42105, 24027, 36657, 25017, 19147, 23050, 34626, 50095, 34965, 32472, 34601, 35714, 38138, 30497, 21496, 17404, 24308, 26553, 31202, 23545, 17783, 19969, 18261, 21858, 18675, 18387, 19188, 16769, 8895, 8395, 15801, 26295, 15720, 10695, 11507, 11974, 11390, 8755, 7871, 8469, 5939], 'list_frequencies': [0.0439453125, 0.048828125, 0.0537109375, 0.05859375, 0.0634765625, 0.068359375, 0.0732421875, 0.078125, 0.0830078125, 0.087890625, 0.0927734375, 0.09765625, 0.1025390625, 0.107421875, 0.1123046875, 0.1171875, 0.1220703125, 0.126953125, 0.1318359375, 0.13671875, 0.1416015625, 0.146484375, 0.1513671875, 0.15625, 0.1611328125, 0.166015625, 0.1708984375, 0.17578125, 0.1806640625, 0.185546875, 0.1904296875, 0.1953125, 0.2001953125, 0.205078125, 0.2099609375, 0.21484375, 0.2197265625, 0.224609375, 0.2294921875, 0.234375, 0.2392578125, 0.244140625, 0.2490234375, 0.25390625, 0.2587890625, 0.263671875, 0.2685546875, 0.2734375, 0.2783203125, 0.283203125, 0.2880859375, 0.29296875, 0.2978515625, 0.302734375, 0.3076171875], 'list_acceleration_energies': [0.0005442224539243258, 0.0005442224539243258, 0.0011791486501693726, 0.004293310469847459, 0.016235969875409054, 0.06836038712905003, 0.18334249781095063, 0.3887260050058365, 0.5590071639226033, 1.4222044450081313, 1.9652477502822876, 1.20273162317276, 0.4203816110757681, 1.1088230152900402, 1.2730270234713188, 0.7264462722466543, 1.1083090274168894, 0.7563785072124921, 0.5789015180716148, 0.6969070868308728, 1.0469025938657615, 1.5146013238521723, 1.057152116748003, 0.9817773068794837, 1.046146729346422, 1.0797978177474095, 1.1530864415425521, 0.9220640098516758, 0.6499225483087393, 0.5262026437832759, 0.7349421894440284, 0.8028188232807013, 0.9433793892970452, 0.7118732043137918, 0.5376615498964603, 0.6037543434674923, 0.5521136795062285, 0.6608674665487729, 0.564630795946488, 0.5559232366836988, 0.5801411358833313, 0.5070036849920566, 0.26893659598093766, 0.25381930559415083, 0.4777366108032373, 0.7950183014411193, 0.4752876097605779, 0.32335884137337023, 0.34790932096151206, 0.36202887018277097, 0.3443718750110039, 0.26470375467263735, 0.23797638526879825, 0.2560566645713953, 0.17956317521425394], 'frequency_resolution': 0.0048828125, 'list_elevation_energies': [0.09362821727097581, 0.06142947335148723, 0.09090717318595427, 0.2337048079537249, 0.6416602902510544, 2.008596522972382, 4.087882868624622, 6.695206865777514, 7.554783512833415, 15.292283722949415, 17.021712062465713, 8.484946006674173, 2.4398710113329507, 5.342836008688601, 5.1348364397885655, 2.4714900619296425, 3.202588735495777, 1.8682978912023542, 1.2295606831874997, 1.2798029039836745, 1.6707621191173565, 2.1106374948852906, 1.2920848793396573, 1.0568501465423867, 0.9957198795211161, 0.9120677780617279, 0.867341294712026, 0.6196570620904839, 0.3914308438589529, 0.284851804553032, 0.35858745052354485, 0.3539793415586025, 0.37683505188968075, 0.25822947965439047, 0.17751517834103206, 0.1818234516576844, 0.15197732227424796, 0.16660321651362436, 0.13060910867363668, 0.11820891020700756, 0.11359250491506193, 0.09156540787831909, 0.04487139122872074, 0.039184256697055414, 0.0683416376733525, 0.10553643574847008, 0.05862813384546467, 0.03711359017924457, 0.03720205901287616, 0.03611033907741153, 0.032078936690433914, 0.02305448556208294, 0.019400586499005407, 0.01956003654212705, 0.012866351575091983], 'wave_spectral_moments': {'m0': 0.4804573044072103, 'm2': 0.005779242270341911, 'm4': 0.0001087473152140824}, 'is_valid': True}]}}

    ds = ta.trajectory_dict_to_dataset(dicts, variable_attributes=var_attrs, global_attributes=config['attributes'])
    comments = [config['drifters'][bname]['comment'] if 'comment' in config['drifters'][bname]
                else '' for bname in config['drifters']]
    comments = [c if c is not None else '' for c in comments]
    if sum([len(c) for c in comments]) > 0:
        ds = ds.assign(drifter_description = (['trajectory'], comments))

    # Add IMU data
    if len(imu) > 0:
        # replace buoy name with trajectory number
        trajs = ds['drifter_names'].values
        for im in imu:
            tn = np.argwhere(trajs == im[0])[0][0]
            im[0] = tn

        imu = [[obs, *m] for obs, m in enumerate(imu)]

        ids = pd.DataFrame(imu, columns=['imu_obs', 'trajectory', 'imu_time', 'Hs', 'Tz', 'Tc', 'm0', 'm2', 'm4', 'is_valid', 'accel_energy_spectrum'])
        ids = ids.set_index((['trajectory', 'imu_obs']))
        ids = ids.to_xarray()

        ids.accel_energy_spectrum.attrs['frequencies'] = list_frequencies
        ids.drop_vars('accel_energy_spectrum')
        ds['imu_obs'] = ids['imu_obs']
        ds['imu_time'] = ids['imu_time']
        ds['Hs'] = ids['Hs']
        ds['Tz'] = ids['Tz']
        ds['Tc'] = ids['Tc']
        ds['m0'] = ids['m0']
        ds['m2'] = ids['m2']
        ds['m4'] = ids['m4']
        ds['imu_is_valid'] = ids['is_valid']

        a = ids['accel_energy_spectrum'].values
        sh = a.shape
        a = [ [np.full((len(list_frequencies),), np.nan) if not isinstance(aa, np.ndarray) else aa for aa in aa] for aa in a ]
        a = np.stack(list(itertools.chain.from_iterable(a)))
        a = a.reshape((*sh, len(list_frequencies)))
        accel = xr.DataArray(data=a, dims = ['trajectory', 'imu_obs', 'frequencies'],
                             coords = dict(trajectory=ds['trajectory'], imu_obs=ds['imu_obs'], frequencies=list_frequencies))
        ds['accel_energy_spectrum'] = accel

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
