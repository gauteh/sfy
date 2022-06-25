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
from sfy.axl import Axl

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
@click.option(
    '-c',
    '--continue',
    'cont',
    default=False,
    is_flag=True,
    type=bool,
    help=
    'Continue to next package in case of error.'
)
@click.option(
        '--start-id',
        default=None,
        type=int,
        help='Skip packages before this id')
@click.option(
        '--stop-id',
        default=None,
        type=int,
        help='Skip packages after this id')
def put(dev, file, really, cont, start_id, stop_id):
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

    packages = b.axl_packages_range()

    if really:
        logger.warning("really posting packages, dry-run is off.")
    else:
        logger.warning("not posting any packages, use --really if you really really wanna do it.")

    uploaded = 0

    for event in collection:
        if start_id is not None:
            if event['body']['storage_id'] < start_id:
                continue

        if stop_id is not None:
            if event['body']['storage_id'] > stop_id:
                continue

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

        # simulate blues notecard JSON where default value is removed
        for key in ['lon', 'lat', 'timestamp', 'position_time']:
            if event['body'].get(key) == 0:
                del event['body'][key]


        # make event id
        hash = hashlib.shake_256()
        hash.update(str(event['body']).encode('utf-8'))
        hash.update(event['payload'].encode('utf-8'))
        hash = hash.hexdigest(length=int((36 - 4) / 2))
        hash = f"{hash[:8]}-{hash[8:12]}-{hash[12:16]}-{hash[16:20]}-{hash[20:]}"
        event['event'] = hash

        new_p = Axl.parse(json.dumps(event))
        uri = f"{int(time):013d}-{event['event']}_axl.qo.json"
        logger.info(f"Event: {uri}, package: {new_p}")

        # check if store id already exists on server
        storage_id = event['body']['storage_id']
        existing_p = next(filter(lambda p: p.storage_id == storage_id, packages), None)

        if existing_p is not None:
            duplicate = new_p.duplicate(existing_p)

            logger.error(f"found package with same storage_id: {storage_id} already on server, package duplicate: {duplicate}")

            logger.info(f"Existing: {existing_p}")
            logger.info(f"New: {new_p}")

            if cont:
                continue
            else:
                raise Exception("storage_id already exists")

        # check if event exists
        try:
            p = b.package(uri)
            logger.error(f"package {p} already exists on server.")
            if cont:
                continue
            else:
                raise Exception("package already exists on server.")

        except requests.exceptions.HTTPError as e:
            # does not exist
            logger.debug("package is new, posting to server..")
            url = urljoin(hub.endpoint, "../buoy")

            logger.info(f"Posting package: {new_p}..")
            if really:
                r = requests.post(
                    url,
                    json=event,
                    headers={'SFY_AUTH_TOKEN': os.getenv('SFY_AUTH_TOKEN')})
                r.raise_for_status()
            uploaded += 1

    logger.info(f"Uploaded {uploaded} packges, dry-run: {not really}")

