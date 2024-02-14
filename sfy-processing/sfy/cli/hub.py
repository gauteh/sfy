import click
import logging
import os
import requests
from datetime import datetime
from sfy.hub import Hub

logger = logging.getLogger(__name__)


@click.group()
def hub():
    pass


@hub.command()
@click.argument('sn')
@click.option('--name', help="optional name of buoy", default=None)
@click.option('--product', help="notehub product", default=None)
def provision(sn, product, name):
    """
    sn: serial number of modem (the dev:... string without the 'dev:' part)
    """

    hub = Hub.from_env()

    if product is None:
        product = os.getenv('SFY_PRODUCT')

    assert product is not None, "product or SFY_PRODUCT env not set."

    logger.info(f"Provisioning {sn} to {product}")

    token = hub.login()

    req_log = logging.getLogger('requests.packages.urllib3')
    req_log.setLevel(logging.DEBUG)
    req_log.propagate = True

    logger.info(f'Getting project for product: {product}')
    r = requests.get(f'https://api.notefile.net/v1/products/{product}/project',
                     headers={'X-SESSION-TOKEN': token})
    r.raise_for_status()

    projectUID = r.json()['uid']
    logger.info(f'Project UID: {projectUID}')


    body = { 'product_uid': product }
    if name is not None:
        body['device_sn'] = name


    logger.info('Provisioning..')
    r = requests.post(f'https://api.notefile.net/v1/projects/{projectUID}/devices/{sn}/provision',
                      json=body,
                     headers={'X-SESSION-TOKEN': token})
    logger.debug(f"Response: {r}: {r.text}")
    if r.status_code is not 200:
        logger.error(f'Error: {r.text}')
    r.raise_for_status()
    logger.info('Done.')
