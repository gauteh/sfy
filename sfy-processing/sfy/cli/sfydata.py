#! /usr/bin/env python
import click
from tabulate import tabulate
from tqdm import tqdm
import coloredlogs

from sfy.hub import Hub
from sfy.cli.track import track
from sfy.cli.axl import axl


@click.group()
def sfy():
    pass


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
        buoys.sort(key=lambda b: b.dev)
        buoys = [[b.dev] for b in buoys]
        print(tabulate(buoys, headers=['Buoys']))
    else:
        buoy = hub.buoy(dev)
        pcks = buoy.packages_range(start, end)
        pcks = [pck for pck in pcks if 'axl.qo.json' in pck[1]]

        # download or fetch from cache
        pcks = [(pck[1], buoy.package(pck[1])) for pck in tqdm(pcks)]

        pcks = [[
            ax[1].start.strftime("%Y-%m-%d %H:%M:%S UTC"), ax[1].lon,
            ax[1].lat, ax[0]
        ] for ax in pcks]
        print(tabulate(pcks, headers=['Time', 'Lon', 'Lat', 'File']))


if __name__ == '__main__':
    coloredlogs.install()
    sfy()
