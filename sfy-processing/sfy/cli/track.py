#! /usr/bin/env python
import click
from tqdm import tqdm
from datetime import timedelta, datetime
import matplotlib.pyplot as plt
from cartopy import crs, feature as cfeature
import pandas as pd
import io
import numpy as np
import logging
from sfy.timeutil import utcify

from sfy.hub import Hub

logger = logging.getLogger(__name__)

@click.group()
def track():
    pass


@track.command(help='Plot track of buoy')
@click.argument('dev')
@click.option('--fast', is_flag=True, help='Plot faster at lower quality.')
@click.option('--start',
              default=None,
              help='Filter packages after this time',
              type=click.DateTime())
@click.option('--end',
              default=None,
              help='Filter packages before this time',
              type=click.DateTime())
@click.option('--margins',
              help='Map limits margins, format: 0.5,0.5',
              default=None,
              type=str)
@click.option(
    '--nib',
    help='Use Norge i Bilder orthophotos (zoom level, higher is better)',
    default=None,
    type=int)
@click.option('--save', help='Save to file', default=None, type=click.File())
def map(dev, fast, nib, start, end, margins, save):
    hub = Hub.from_env()
    buoy = hub.buoy(dev)
    print(buoy)

    start = start if start is not None else datetime.now() - timedelta(days=1)
    end = end if end is not None else datetime.now()

    pcks = buoy.position_packages_range(start - timedelta(days=1),
                                        end + timedelta(days=1))
    pcks = [
        p for p in pcks if p.best_position_time and p.best_position_time >= utcify(start)
        and p.best_position_time <= utcify(end)
    ]
    pcks.sort(key=lambda p: p.best_position_time)

    lon = [pck.longitude for pck in pcks]
    lat = [pck.latitude for pck in pcks]
    tm = np.array([pck.best_position_time for pck in pcks])

    print('plotting..')
    fig = plt.figure()

    if nib is not None:
        logger.debug(f'map: adding NIB images (level: {nib})')
        from plz.map import NIB
        img = NIB(cache=True)
        ax = fig.add_subplot(1, 1, 1, projection=img.crs)
        ax.add_image(img, nib)
    else:
        ax = fig.add_subplot(1, 1, 1, projection=crs.Mercator())

    if fast:
        # ax.stock_img()
        logger.debug('map: adding natural earth feature')
        ax.coastlines(resolution='10m')
        nh = cfeature.NaturalEarthFeature(name='land',
                                          category='physical',
                                          scale='10m',
                                          facecolor=cfeature.COLORS['land'])
        ax.add_feature(nh, zorder=-1)

    if not fast and nib is None:
        logger.debug('map: adding GSHHG features')
        gsh = cfeature.GSHHSFeature(levels=[1],
                                    facecolor=cfeature.COLORS['land'])
        ax.add_feature(gsh, zorder=-1)

    if nib is None:
        ax.gridlines(crs.PlateCarree(), draw_labels=True)

    ax.plot(lon,
            lat,
            '-o',
            gid=tm,
            transform=crs.PlateCarree(),
            label=buoy.dev,
            picker=True)
    ax.plot(lon[0], lat[0], '*', transform=crs.PlateCarree())
    ax.plot(lon[-1], lat[-1], 'X', transform=crs.PlateCarree())

    from matplotlib.lines import Line2D

    def onpick1(event):
        if isinstance(event.artist, Line2D):
            thisline = event.artist
            xdata = thisline.get_xdata()
            ydata = thisline.get_ydata()
            ind = event.ind
            print(ind)
            print('onpick1 line:', xdata[ind], ydata[ind], tm[ind])

    fig.canvas.mpl_connect('pick_event', onpick1)

    if margins is not None:
        ms = margins.split(',')
        mx = float(ms[0])
        my = float(ms[1])
        margins = (mx, my)
        ax.margins(*margins)

    plt.legend()
    plt.title(f'Track of {buoy.dev}')

    if save is not None:
        plt.savefig(save)
    else:
        plt.show()


@track.command(help='Output CSV of position')
@click.argument('dev')
@click.option('--start',
              default=None,
              help='Filter packages after this time',
              type=click.DateTime())
@click.option('--end',
              default=None,
              help='Filter packages before this time',
              type=click.DateTime())
@click.option('--tower',
              default=False,
              is_flag=True,
              help='Include positions based on cell tower',
              type=bool)
@click.option('--axl',
              default=False,
              is_flag=True,
              help='Include positions from acceleration packages',
              type=bool)
def csv(dev, start, end, tower, axl):
    hub = Hub.from_env()
    buoy = hub.buoy(dev)

    pcks = buoy.position_packages_range(start, end)

    tm = [pck.best_position_time for pck in pcks]
    lon = [pck.longitude for pck in pcks]
    lat = [pck.latitude for pck in pcks]
    typ = [pck.position_type for pck in pcks]
    file = [pck.file for pck in pcks]
    bearing = [pck.body.get('bearing', None) for pck in pcks]
    velocity = [pck.body.get('velocity', None) for pck in pcks]
    distance = [pck.body.get('distance', None) for pck in pcks]
    temperature = [pck.body.get('temperature', None) for pck in pcks]
    voltage = [pck.body.get('voltage', None) for pck in pcks]
    tower_lat = [ pck.tower_lat for pck in pcks ]
    tower_lon = [ pck.tower_lon for pck in pcks ]

    df = pd.DataFrame({
        'Device': buoy.dev,
        'Time': tm,
        'Type': typ,
        'Longitude': lon,
        'Latitude': lat,
        'File': file,
        'Bearing': bearing,
        'Velocity': velocity,
        'Distance': distance,
        'Temperature': temperature,
        'Voltage': voltage,
        'TowerLat' : tower_lat,
        'TowerLon' : tower_lon,
    })

    if not tower:
        df = df[df['Type'] == 'gps']

    if not axl:
        df = df[df['File'] == '_track.qo']

    buf = io.StringIO()
    df.to_csv(buf, index=False)
    print(buf.getvalue())


@track.command(help="Plot stats, voltage, temperature, etc")
@click.argument('dev')
@click.option('--start',
              default=None,
              help='Filter packages after this time',
              type=click.DateTime())
@click.option('--end',
              default=None,
              help='Filter packages before this time',
              type=click.DateTime())
def stats(dev, start, end):
    """
    Plot stats, voltage, temperature, etc
    """
    hub = Hub.from_env()
    buoy = hub.buoy(dev)

    pcks = buoy.position_packages_range(start, end)
    pcks = [pck for pck in pcks if pck.file == '_track.qo']

    tm = np.array([pck.best_position_time for pck in pcks], dtype='datetime64[s]')
    lon = [pck.longitude for pck in pcks]
    lat = [pck.latitude for pck in pcks]
    typ = [pck.position_type for pck in pcks]
    file = [pck.file for pck in pcks]
    bearing = [pck.body.get('bearing', None) for pck in pcks]
    velocity = [pck.body.get('velocity', None) for pck in pcks]
    distance = [pck.body.get('distance', None) for pck in pcks]
    temperature = [pck.body.get('temperature', None) for pck in pcks]
    voltage = [pck.body.get('voltage', None) for pck in pcks]

    f = plt.figure()
    ax = plt.gca()
    ax.plot(tm, voltage, color='C2', label='Voltage [V]')
    ax.grid()
    ax.set_ylim([3., 5.])

    at = ax.twinx()
    at.plot(tm, temperature, color='C1', label='Temperature (modem) [C]')

    plt.title(f'Statistics for {buoy.name} ({buoy.dev})')
    f.legend()
    plt.show()


if __name__ == '__main__':
    track()
