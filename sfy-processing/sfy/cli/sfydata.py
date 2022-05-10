#! /usr/bin/env python
import click
from tabulate import tabulate
from tqdm import tqdm
from datetime import datetime, timezone
import coloredlogs
import logging

logger = logging.getLogger(__name__)

from sfy.hub import Hub
from sfy.cli.track import track
from sfy.cli.axl import axl


@click.group()
def sfy():
    coloredlogs.install()


sfy.add_command(track)
sfy.add_command(axl)


@sfy.command(help='List available buoys or data')
@click.argument('dev', default=None, required=False)
@click.option('--start',
              default=None,
              help='Filter packages after this time',
              type=click.DateTime())
@click.option('--end',
              default=None,
              help='Filter packages before this time',
              type=click.DateTime())
def list(dev, start, end):
    hub = Hub.from_env()

    if dev is None:
        buoys = hub.buoys()

        last = [b.last() if 'lost+found' not in b.dev else None for b in buoys]
        last = [l.received_datetime if l else None for l in last]

        buoys = [[b.dev, b.name, l] for b, l in zip(buoys, last)]
        buoys.sort(key=lambda b: b[2].timestamp() if b[2] else 0)

        print(tabulate(buoys, headers=['Buoys', 'Name', 'Last contact']))
    else:
        buoy = hub.buoy(dev)
        logger.info(f"Listing packages for {buoy}")
        pcks = buoy.packages_range(start, end)
        pcks = [pck for pck in pcks if 'axl.qo.json' in pck[1]]

        # download or fetch from cache
        pcks = [(pck[1], buoy.package(pck[1])) for pck in tqdm(pcks)]

        pcks = [[
            ax[1].start.strftime("%Y-%m-%d %H:%M:%S UTC"), ax[1].lon,
            ax[1].lat,
            datetime.fromtimestamp(float(ax[0].split('-')[0]) / 1000., tz=timezone.utc).strftime("%Y-%m-%d %H:%M:%S UTC"),
            ax[0]
        ] for ax in pcks if ax[1] is not None]
        print(tabulate(pcks, headers=['DataTime', 'Lon', 'Lat', 'TxTime', 'File']))


if __name__ == '__main__':
    sfy()
