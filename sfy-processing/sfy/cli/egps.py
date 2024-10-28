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
from sfy.egps import Egps, EgpsCollection
from sfy import signal
from sfy.timeutil import utcify
import logging

logger = logging.getLogger(__name__)


@click.group()
def egps():
    pass


@egps.command(name='list', help='List egps packages')
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
    pcks = buoy.egps_packages_range(tx_start, tx_end)

    pcks = [[
        ax.start.strftime("%Y-%m-%d %H:%M:%S UTC"), ax.longitude, ax.latitude,
        ax.received_datetime.strftime("%Y-%m-%d %H:%M:%S UTC"), ax.fname
    ] for ax in pcks]
    print(tabulate(pcks, headers=['DataTime', 'Lon', 'Lat', 'TxTime', 'File']))


@egps.command()
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
def ts(dev, tx_start, tx_end, start, end, file, gap, freq):
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

    pcks = buoy.egps_packages_range(tx_start, tx_end)
    logger.info(f"{len(pcks)} packages in tx range")

    if freq:
        pcks = list(filter(lambda p: abs(p.frequency - freq) <= 2, pcks))
        logger.info(
            f"Filtering packages on frequency: {freq}, {len(pcks)} packages matching."
        )

    logger.debug(f'Building collection..')
    pcks = EgpsCollection(pcks)

    # filter packages between start and end
    pcks.clip(start, end)
    logger.info(
        f"{len(pcks)} in start <-> end range, splitting into segments..")

    gap = gap if gap is not None else EgpsCollection.GAP_LIMIT

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
                 ]))

    del segments

    if file:
        logger.info(f"Saving to {file}..")

        if len(pcks) > 0:
            pcks.to_netcdf(file)
        else:
            logger.error("No data to save.")
