#! /usr/bin/env python

import click
import matplotlib.pyplot as plt
from datetime import timedelta, datetime
from tabulate import tabulate
import numpy as np
import time
import os

import sfy
from sfy.hub import Hub
from sfy.axl import Axl, AxlCollection
from sfy import signal
from sfy.timeutil import utcify
import logging

logger = logging.getLogger(__name__)


@click.group()
def axl():
    pass


@axl.command(name='list', help='List axl packages')
@click.argument('dev')
@click.option('--tx-start',
              default=None,
              help='Filter packages sent after this time',
              type=click.DateTime())
@click.option('--tx-end',
              default=None,
              help='Filter packages sent before this time',
              type=click.DateTime())
def list_buoys(dev, tx_start, tx_end):
    hub = Hub.from_env()
    buoy = hub.buoy(dev)
    logger.info(f"Listing packages for {buoy}")
    pcks = buoy.axl_packages_range(tx_start, tx_end)

    pcks = [[
        ax.start.strftime("%Y-%m-%d %H:%M:%S UTC"), ax.lon, ax.lat,
        ax.received_datetime.strftime("%Y-%m-%d %H:%M:%S UTC"), ax.storage_id,
        ax.fname
    ] for ax in pcks]
    print(
        tabulate(pcks,
                 headers=['DataTime', 'Lon', 'Lat', 'TxTime', 'StID', 'File']))


@axl.command()
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
@click.option('--file',
              default=None,
              help='Store to this file',
              type=click.Path())
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
@click.option('--displacement',
              default=False,
              is_flag=True,
              help='Include integrated displacement',
              type=bool)
@click.option('--no-retime',
              default=False,
              is_flag=True,
              help='Do not retime based on estimated frequency.',
              type=bool)
def ts(dev, tx_start, tx_end, start, end, file, gap, freq, displacement,
       no_retime):
    hub = Hub.from_env()
    buoy = hub.buoy(dev)

    if tx_start is None:
        tx_start = datetime.utcnow() - timedelta(days=1)

    if tx_end is None:
        if end is not None:
            tx_end = end + timedelta(days=14)
        else:
            tx_end = datetime.utcnow()

    if start is None:
        start = tx_start

    if end is None:
        end = datetime.utcnow()

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

    logger.debug(f'Building collection..')
    pcks = AxlCollection(pcks)

    # filter packages between start and end
    pcks.clip(start, end)
    logger.info(
        f"{len(pcks)} in start <-> end range, splitting into segments..")

    gap = gap if gap is not None else AxlCollection.GAP_LIMIT

    logger.debug('Splitting collection into segments..')
    segments = list(pcks.segments(eps_gap=gap))
    logger.info(f"Collection consists of: {len(segments)} segments")

    assert len(pcks) == sum(len(s) for s in segments)

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

    del segments

    if file:
        logger.info(f"Saving to {file}..")

        if len(pcks) > 0:
            pcks.to_netcdf(file, displacement=displacement, retime=(not no_retime))
        else:
            logger.error("No data to save.")


@axl.command()
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
@click.option('--file',
              default=None,
              help='Store to this file',
              type=click.Path())
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
@click.option('--raw',
              default=False,
              is_flag=True,
              help='Do not try to filter automatically',
              type=bool)
@click.option('--no-retime',
              default=False,
              is_flag=True,
              help='Do not retime based on estimated frequency.',
              type=bool)
def stats(dev, tx_start, tx_end, start, end, file, gap, freq, raw, no_retime):
    hub = Hub.from_env()
    buoy = hub.buoy(dev)

    if tx_start is None:
        tx_start = datetime.utcnow() - timedelta(days=1)

    if tx_end is None:
        if end is not None:
            tx_end = end + timedelta(days=14)
        else:
            tx_end = datetime.utcnow()

    if start is None:
        start = tx_start

    if end is None:
        end = datetime.utcnow()

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

    logger.debug(f'Building collection..')
    pcks = AxlCollection(pcks)

    # filter packages between start and end
    pcks.clip(start, end)
    logger.info(
        f"{len(pcks)} in start <-> end range, splitting into segments..")

    gap = gap if gap is not None else AxlCollection.GAP_LIMIT

    logger.debug('Splitting collection into segments..')
    segments = list(pcks.segments(eps_gap=gap))
    logger.info(f"Collection consists of: {len(segments)} segments")

    assert len(pcks) == sum(len(s) for s in segments)

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

    del segments

    if len(pcks) > 0:
        ds = pcks.to_dataset(retime=(not no_retime))
        st = sfy.xr.spec_stats(ds, raw=raw)
        print(st)

        if file:
            logger.info(f"Saving to {file}..")

            compression = {'zlib': True}
            encoding = {}
            for v in st.variables:
                encoding[v] = compression.copy()
            encoding['time']['units'] = 'microseconds since 2023-01-01 00:00:00.0'
            st.to_netcdf(file, format='NETCDF4', encoding=encoding)
    else:
        logger.error("No data.")


@axl.command()
@click.argument('dev')
@click.argument('raw_files', type=click.Path(), nargs=-1, required=True)
@click.option('--start',
              default=None,
              help='Clip results before this (default: tx-start)',
              type=click.DateTime())
@click.option('--end',
              default=None,
              help='Clip results after this (default: tx-end)',
              type=click.DateTime())
@click.option('--file',
              default=None,
              help='Store to this file',
              type=click.Path())
@click.option(
    '--gap',
    default=None,
    help=
    'Maximum gap allowed between packages before splitting into new segment (seconds).',
    type=float)
@click.option('--displacement',
              default=False,
              is_flag=True,
              help='Include integrated displacement',
              type=bool)
@click.option('--raw',
              default=False,
              is_flag=True,
              help='Files contain raw accelerometer and gyroscope readings.',
              type=bool)
@click.option('--no-retime',
              default=False,
              is_flag=True,
              help='Do not retime based on estimated frequency.',
              type=bool)
def raw(dev, start, end, gap, displacement, raw, file, raw_files, no_retime):
    """
    Read raw files from SD-card and collect into a collection.
    """
    hub = Hub.from_env()
    b = hub.buoy(dev)
    logger.info(f"Reading {len(raw_files)} raw files..")

    pcks = [
        AxlCollection.from_storage_file(b.name, b.dev, f, raw=raw)
        for f in raw_files
    ]
    pcks = [p.pcks for p in pcks if p is not None]
    pcks = [p for li in pcks for p in li]
    logger.info(f"{len(pcks)} packages in files")

    pcks = AxlCollection(pcks)

    # filter packages between start and end
    if start is not None and end is not None:
        pcks.clip(utcify(start), utcify(end))
        logger.info(
            f"{len(pcks)} in start <-> end range, splitting into segments..")

    gap = gap if gap is not None else AxlCollection.GAP_LIMIT

    segments = list(pcks.segments(eps_gap=gap))
    logger.info(f"Collection consists of: {len(segments)} segments")

    assert len(pcks) == sum(len(s) for s in segments)

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

    if file:
        logger.info(f"Saving to {file}..")

        pcks.to_netcdf(file, displacement=displacement, retime=(not no_retime))


@axl.command(help='Monitor buoy')
@click.argument('dev')
@click.option('--sleep',
              help='Time to sleep between update',
              default=5.0,
              type=float)
@click.option('--window',
              help='Time window to show (seconds).',
              default=60.0,
              type=float)
@click.option('--delay',
              help='Delay in data, use to re-play data in the past (seconds).',
              default=0.0,
              type=float)
@click.option('--ylim', help='Height of y-axis (m)', default=1.0, type=float)
def monitor(dev, sleep, window, delay, ylim):
    hub = Hub.from_env()
    buoy = hub.buoy(dev)

    plt.ion()
    fig = plt.figure()
    ax = fig.add_subplot(111)
    plt.grid()
    plt.xlabel('Time')
    plt.ylabel('Vertical displacement ($m$)')

    la, = ax.plot([], [])

    while True:
        # Get packages from time-window
        end = datetime.utcnow() - timedelta(seconds=delay)
        start = end - timedelta(seconds=window)
        start, end = utcify(start), utcify(end)

        logger.info(f"Getting packages in window.. {start} -> {end}")
        pcks = buoy.axl_packages_range(start - timedelta(minutes=20), end)
        if len(pcks) > 0:
            logger.info(f"Last package: {pcks[-1]}")

            pcks = AxlCollection(pcks)
            logger.debug(f"{len(pcks)} packages in tx range")

            pcks.clip(
                start - timedelta(seconds=window), end
            )  # add some margin to start, so that we get a full trace for the window.
            logger.info(f"{len(pcks)} in start <-> end range")

            if len(pcks) > 0:
                plt.title(
                    f"Buoy: {buoy.dev}\n{pcks.start} -> {pcks.end} length: {pcks.duration}s f={pcks.frequency}Hz"
                )

                for p in pcks.pcks:
                    logger.debug(f"{p}")

                logger.debug("Integrating to displacement..")
                wz = signal.integrate(pcks.z,
                                      pcks.dt,
                                      order=2,
                                      method='dft',
                                      freqs=[0.1, 25])

                logger.debug("Plotting..")
                la.set_data(pcks.time[:-1], wz)

                x1 = pcks.time[-2]
                x0 = x1 - timedelta(seconds=window)
                plt.xlim([x0, x1])
                plt.ylim([-ylim, ylim])

        plt.pause(sleep)
