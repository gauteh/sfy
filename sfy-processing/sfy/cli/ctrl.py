import click
import logging
import os
import requests
from datetime import datetime
from sfy.hub import Hub

logger = logging.getLogger(__name__)


@click.group()
def ctrl():
    pass


@ctrl.command()
@click.argument('dev')
@click.argument('start', type=int)
@click.argument('end', type=int)
def get_pcks(dev, start, end):
    """
    dev:    Device
    start:  Start storage ID
    end:    End storage ID
    """

    hub = Hub.from_env()
    b = hub.buoy(dev)

    logger.info(f"Requesting packages from {b}")

    token = hub.login()

    product = os.getenv('SFY_PRODUCT')
    assert product is not None, "SFY_PRODUCT env not set."

    req_log = logging.getLogger('requests.packages.urllib3')
    req_log.setLevel(logging.DEBUG)
    req_log.propagate = True

    logger.debug("Deleting current note..")
    r = requests.post(
        f'https://api.notefile.net/req?product={product}&device=dev:{b.dev[3:]}',
        json={
            'req': 'note.delete',
            'file': 'storage.db',
            'note': 'request-data',
        },
        headers={'X-SESSION-TOKEN': token})
    logger.debug(f"Response: {r}: {r.text}")
    r.raise_for_status()

    logger.debug("Updating note..")
    r = requests.post(
        f'https://api.notefile.net/req?product={product}&device=dev:{b.dev[3:]}',
        json={
            'req': 'note.update',
            'file': 'storage.db',
            'note': 'request-data',
            'body': {'request_start' : start, 'request_end': end }
        },
        headers={'X-SESSION-TOKEN': token})
    logger.debug(f"Response: {r}: {r.text}")
    r.raise_for_status()


@ctrl.command()
@click.argument('dev')
def clear_get(dev):
    hub = Hub.from_env()
    b = hub.buoy(dev)
    logger.info(f"Clearing request for buoy: {b}")

    token = hub.login()

    product = os.getenv('SFY_PRODUCT')
    assert product is not None, "SFY_PRODUCT env not set."

    req_log = logging.getLogger('requests.packages.urllib3')
    req_log.setLevel(logging.DEBUG)
    req_log.propagate = True

    logger.debug("Deleting current note..")
    r = requests.post(
        f'https://api.notefile.net/req?product={product}&device=dev:{b.dev[3:]}',
        json={
            'req': 'note.delete',
            'file': 'storage.db',
            'note': 'request-data',
        },
        headers={'X-SESSION-TOKEN': token})
    logger.debug(f"Response: {r}: {r.text}")
    r.raise_for_status()

@ctrl.command()
@click.argument('dev')
def status(dev):
    hub = Hub.from_env()
    b = hub.buoy(dev)
    logger.info(f"Getting current storage-info for: {b}")

    token = hub.login()

    product = os.getenv('SFY_PRODUCT')
    assert product is not None, "SFY_PRODUCT env not set."

    req_log = logging.getLogger('requests.packages.urllib3')
    req_log.setLevel(logging.DEBUG)
    req_log.propagate = True

    logger.debug("Getting request-data..")
    rd = requests.post(
        f'https://api.notefile.net/req?product={product}&device=dev:{b.dev[3:]}',
        json={
            'req': 'note.get',
            'file': 'storage.db',
            'note': 'request-data',
        },
        headers={'X-SESSION-TOKEN': token})
    logger.debug(f"Response: {rd}: {rd.text}")
    rd.raise_for_status()

    request_start = None
    request_end = None
    request_time = None

    rdb = rd.json().get('body')
    if rdb is not None:
        request_start = rdb.get('request_start')
        request_end = rdb.get('request_end')
        request_time = datetime.utcfromtimestamp(rd.json().get('time'))

    print(f"Buoy: {b.name} / {b.dev}")
    print()
    print("Request-data:")
    print("request_start ....: %s" % request_start)
    print("request_end ......: %s" % request_end)
    print("time .............: %s" % request_time)
