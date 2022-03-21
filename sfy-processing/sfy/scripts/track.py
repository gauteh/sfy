#! /usr/bin/env python
import click
from tabulate import tabulate
from tqdm import tqdm

from sfy.hub import Hub


@click.group()
def track():
    pass


@track.command(help='List available buoys')
def list():
    hub = Hub.from_env()
    buoys = hub.buoys()
    buoys.sort(key=lambda b: b.dev)
    buoys = [[b.dev] for b in buoys]
    print(tabulate(buoys, headers=['Buoy']))


@track.command(help='Plot track of buoy')
@click.argument('dev')
@click.option('--start',
              default=None,
              help='Filter packages after this time',
              type=click.DateTime())
@click.option('--end',
              default=None,
              help='Filter packages before this time',
              type=click.DateTime())
def plot(dev, start, end):
    hub = Hub.from_env()
    buoy = hub.buoy(dev)
    print(buoy)

    pcks = buoy.packages_range(start, end)
    pcks = [pck for pck in pcks if 'axl.qo.json' in pck[1]]

    pcks = [buoy.package(pck[1]) for pck in tqdm(pcks)]



if __name__ == '__main__':
    track()
