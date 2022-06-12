import click
import logging
import os
import requests
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
    info = b.storage_info()

    logger.info(f"Requesting packages from {b}: {info}")

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
    info = b.storage_info()
    logger.info(f"Clearing request for buoy: {b}: {info}")

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

