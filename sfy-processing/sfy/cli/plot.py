#! /usr/bin/env python

import click
import matplotlib.pyplot as plt
from tabulate import tabulate
from datetime import timedelta, datetime
import numpy as np

from sfy.hub import Hub
from sfy.axl import AxlCollection
from sfy import signal
import sfy.xr
from sfy.timeutil import utcify
import logging

logger = logging.getLogger(__name__)


@click.group()
@click.argument('dev')
@click.option('--tx-start',
              default=None,
              help='Search in packages after this time (default: 24h ago)',
              type=click.DateTime())
@click.option('--tx-end',
              default=None,
              help='Search in packages before this time (default: now)',
              type=click.DateTime())
@click.option('--start',
              default=None,
              help='Clip results before this (default: tx-start)',
              type=click.DateTime())
@click.option('--end',
              default=None,
              help='Clip results after this (default: tx-end)',
              type=click.DateTime())
@click.option(
    '--gap',
    default=None,
    help=
    'Maximum gap allowed between packages before splitting into new segment (seconds).',
    type=float)
@click.option(
    '--freq',
    default=None,
    help=
    'Only use packages with this frequency (usually 52 or 20.8, within 2 Hz)',
    type=float)
@click.option(
        '--f0',
        type=float,
        default=None,
        help='Lower cut-off frequency')
@click.option(
        '--f1',
        type=float,
        default=None,
        help='Upper cut-off frequency')
@click.pass_context
def plot(ctx, dev, tx_start, tx_end, start, end, gap, freq, f0, f1):
    hub = Hub.from_env()
    buoy = hub.buoy(dev)

    if tx_start is None:
        tx_start = datetime.utcnow() - timedelta(days=1)

    if tx_end is None:
        tx_end = datetime.utcnow()

    if start is None:
        start = tx_start

    if end is None:
        end = tx_end

    if tx_start > start:
        tx_start = start

    if tx_end < end:
        tx_end = end

    tx_start = utcify(tx_start)
    tx_end = utcify(tx_end)
    start = utcify(start)
    end = utcify(end)

    logger.info(
        f"Scanning for packages tx: {tx_start} <-> {tx_end} and clipping between {start} <-> {end}"
    )

    pcks = buoy.axl_packages_range(tx_start, tx_end)
    logger.info(f"{len(pcks)} packages in tx range")

    if freq:
        pcks = list(filter(lambda p: abs(p.frequency - freq) <= 2, pcks))
        logger.info(
            f"Filtering packages on frequency: {freq}, {len(pcks)} packages matching."
        )

    pcks = AxlCollection(pcks)

    # filter packages between start and end
    pcks.clip(start, end)
    logger.info(
        f"{len(pcks)} in {pcks.start} <-> {pcks.end} range, splitting into segments.."
    )

    gap = gap if gap is not None else AxlCollection.GAP_LIMIT

    segments = list(pcks.segments(eps_gap=gap))
    logger.info(f"Collection consists of: {len(segments)} segments")

    stable = [[
        s.start,
        s.end,
        s.duration,
        timedelta(seconds=s.duration),
        s.max_gap(),
        np.nan,
        len(s),
        s.pcks[0].storage_id,
        s.pcks[-1].storage_id,
    ] for s in segments]

    for i, _ in enumerate(stable[1:]):
        stable[i + 1][5] = (stable[i + 1][0] - stable[i][1])

    print(
        tabulate(stable,
                 headers=[
                     'Start',
                     'End',
                     'Duration (s)',
                     'Duration',
                     'Max Internal Gap',
                     'Segment Gap',
                     'Packages',
                     'Start ID',
                     'End ID',
                 ]))

    ctx.ensure_object(dict)
    ctx.obj['pcks'] = pcks
    ctx.obj['buoy'] = buoy

    freqs = pcks.default_bandpass_freqs()
    if f0 is not None:
        freqs[0] = f0
    if f1 is not None:
        freqs[1] = f1

    ctx.obj['freqs'] = freqs


@plot.command(help='Plot timeseries')
@click.pass_context
def ts(ctx):
    logger.info('Making dataset..')
    c = ctx.obj['pcks']
    f = ctx.obj['freqs']

    ds = c.to_dataset(displacement=True, filter_freqs=f)

    logger.info('Plotting..')

    plt.figure()
    ds.u_z.plot()
    plt.grid()
    plt.show()


@plot.command(help='Plot Hs')
@click.pass_context
@click.option('--raw',
              is_flag=True,
              help='Do not attempt to cut away low frequency noise',
              default=False)
def hm0(ctx, raw):
    logger.info('Calculating Hm0..')
    c = ctx.obj['pcks']
    ds = c.to_dataset(retime=False)
    hm0 = sfy.xr.hm0(ds, raw)

    logger.info('Plotting..')
    plt.figure()
    hm0.plot(label='Hm0')
    plt.grid()
    plt.title('Significant wave height for 20 minute windows')
    plt.legend()
    plt.show()


@plot.command(help='Plot Welch spectrum')
@click.pass_context
@click.option('--loglog',
              is_flag=True,
              help='Use logarithmic scales',
              default=False)
@click.option('--acceleration',
              is_flag=True,
              help='Plot the acceleration spectrum as well',
              default=False)
@click.option('--raw',
              is_flag=True,
              help='Do not attempt to cut away low frequency noise',
              default=False)
def welch(ctx, loglog, acceleration, raw):
    logger.info('Calculating Welch spectrum..')
    c = ctx.obj['pcks']
    f, P = signal.welch(c.frequency, c.z)

    ci, cf, PP = signal.imu_cutoff_rabault2022(f, P)

    if not raw:
        P = PP

    hm0 = signal.hm0(f, P)
    phm0 = signal.hm0(f, PP)

    logger.info('Plotting..')
    plt.figure()

    l = f'Elevation (hm0 = {phm0:0.2f}m, raw hm0 = {hm0:0.2f}m'

    if loglog:
        plt.loglog(f, P, label=l)
    else:
        plt.plot(f, P, label=l)

    logger.info(f'Cut-off: {cf} Hz, E={P[ci]} m^2/Hz')
    plt.plot(cf, P[ci], 'x', label='Cut-off')

    if acceleration:
        fa, PA = signal.welch(c.frequency, c.z, order=0)
        if loglog:
            plt.loglog(fa, PA, label='Acceleration')
        else:
            plt.plot(fa, PA, label='Acceleration')

        ci, cf, PPA = signal.imu_cutoff_rabault2022(f, PA)
        logger.info(f'Cut-off (acceleration): {cf} Hz, E={P[ci]} m^2/s')
        plt.plot(cf, PA[ci], 'x', label='Cut-off (acceleration)')

    plt.legend()
    plt.grid()
    plt.title('Elevation energy (Welch)')
    plt.xlabel('Frequency [Hz]')
    plt.ylabel('Energy [m^2/Hz]')
    plt.legend()
    plt.show()
