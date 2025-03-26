#! /usr/bin/env python
import click
from tabulate import tabulate
from tqdm import tqdm
from datetime import datetime, timezone
import coloredlogs
import logging
import json as pyjson

logger = logging.getLogger(__name__)

from sfy.hub import Hub
from sfy.cli.track import track
from sfy.cli.axl import axl
from sfy.cli.egps import egps
from sfy.cli.ctrl import ctrl
from sfy.cli.hub import hub
from sfy.cli.store import store
from sfy.cli.collection import collection
from sfy.cli.plot import plot
from sfy.cli.omb import omb


@click.group()
@click.option('--log', default='info', type=str, help='Python log level')
def sfy(log):
    coloredlogs.install(level=log, logger=logging.getLogger("sfy"))


sfy.add_command(track)
sfy.add_command(axl)
sfy.add_command(egps)
sfy.add_command(ctrl)
sfy.add_command(hub)
sfy.add_command(store)
sfy.add_command(collection)
sfy.add_command(plot)
sfy.add_command(omb)


@sfy.command(help='List available buoys')
def list():
    hub = Hub.from_env()
    buoys = hub.buoys()

    last = [b.last() if 'lost+found' not in b.dev else None for b in buoys]
    storage_info = [ l.body.get('storage_id', None) if l and l.body else None for l in last ]
    last = [l.received_datetime if l else None for l in last]

    buoys = [[b.dev, b.name, l, si]
                for b, l, si in zip(buoys, last, storage_info)]
    buoys.sort(key=lambda b: b[2].timestamp() if b[2] else 0)

    print(
        tabulate(buoys,
                    headers=[
                        'Buoys',
                        'Name',
                        'Last contact',
                        'Last SD-card ID',
                    ]))


@sfy.command(help='Print JSON')
@click.argument('dev')
@click.argument('file')
def json(dev, file):
    hub = Hub.from_env()
    buoy = hub.buoy(dev)
    pck = buoy.json_package(file)
    print(pyjson.dumps(pck))

@sfy.command(help='List all events')
@click.argument('dev')
def events(dev):
    hub = Hub.from_env()
    buoy = hub.buoy(dev)
    packages = buoy.packages()
    print(packages)

@sfy.command(help='Show log messages')
@click.argument('dev')
@click.option('--start',
              default=None,
              help='Filter packages after this time',
              type=click.DateTime())
@click.option('--end',
              default=None,
              help='Filter packages before this time',
              type=click.DateTime())
def log(dev, start, end):
    import json

    hub = Hub.from_env()
    buoy = hub.buoy(dev)
    logger.info(f'Fetching log entries for {buoy}')

    pcks = buoy.fetch_packages_range(start, end)
    # pcks = buoy.packages_range(start, end)
    pcks = [p for p in pcks if 'health.qo' in p[1]]
    pcks = [p[2] for p in tqdm(pcks)]

    pcks = [json.loads(p) for p in pcks]
    pcks.sort(key=lambda p: p.get('received', 0))
    pcks = [[datetime.utcfromtimestamp(p.get('when', 0)), p['body']['text']]
            for p in pcks]
    print(tabulate(pcks, headers=['Time', 'Message']))


if __name__ == '__main__':
    sfy()
