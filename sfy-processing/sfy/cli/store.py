import click
import logging
import os
import subprocess
import requests
from urllib.parse import urljoin
import hashlib
import json
from datetime import datetime
from sfy.hub import Hub

logger = logging.getLogger(__name__)


@click.group()
def store():
    pass


@store.command()
@click.argument('dev')
@click.argument('file', type=click.Path())
@click.option(
    '--really',
    default=False,
    is_flag=True,
    type=bool,
    help=
    'Actually post event to server, otherwise just parse, verify and check for duplicate.'
)
def put(dev, file, really):
    """
    Put data from storage file to server.
    """
    hub = Hub.from_env()
    b = hub.buoy(dev)
    logger.info(f"Putting packages in {file} to {b}")

    logger.info("Parsing collection..")
    collection = subprocess.check_output(["sfypack", "--note", file])
    collection = json.loads(collection)
    logger.info(f"Read {len(collection)} packages.")

    if really:
        logger.warning("really posting packages, dry-run is off.")
    else:
        logger.warning("not posting any packages, use --really to really do it.")

    for event in collection:
        time, lat, lon = event['body']['timestamp'], event['body'][
            'lat'], event['body']['lon']
        time_s = time / 1.e3

        event['device'] = "dev:" + b.dev[3:]
        event['sn'] = b.name
        event['received'] = time_s
        event['when'] = int(time_s)
        event['from_store'] = True
        event['file'] = "axl.qo"
        event['where_when'] = int(time_s)
        event['where_lat'] = lat
        event['where_lon'] = lon
        event['where_timezone'] = 'UTC'
        event['tower_when'] = int(time_s)
        event['tower_lat'] = lat
        event['tower_lon'] = lon
        event['tower_timezone'] = 'UTC'

        # make event id
        hash = hashlib.shake_256()
        hash.update(str(event['body']).encode('utf-8'))
        hash.update(event['payload'].encode('utf-8'))
        hash = hash.hexdigest(length=int((36 - 4) / 2))
        hash = f"{hash[:8]}-{hash[8:12]}-{hash[12:16]}-{hash[16:20]}-{hash[20:]}"
        event['event'] = hash

        uri = f"{int(time)}-{event['event']}_axl.qo.json"
        logger.info(f"Event: {uri}")

        # check if event exists
        try:
            p = b.package(uri)
            logger.error("package already exists on server, skipping.")
        except requests.exceptions.HTTPError as e:
            # does not exist
            logger.debug("package is new, posting to server..")
            url = urljoin(hub.endpoint, "../buoy")

            if really:
                r = requests.post(
                    url,
                    json=event,
                    headers={'SFY_AUTH_TOKEN': os.getenv('SFY_AUTH_TOKEN')})
                r.raise_for_status()
