import coloredlogs
import logging
import click
import requests
from tqdm import tqdm

logger = logging.getLogger(__name__)

import sfy

@click.command()
@click.argument('target')
@click.argument('target-token')
def migrate(target, target_token):
    coloredlogs.install('info')

    hub = sfy.hub.Hub.from_env()

    for buoy in hub.buoys():
        logger.info(f"Migrating {buoy}")

        ep = f"{target}"

        for pck in tqdm(buoy.packages()):
            received = pck.split('-')[0]
            # logger.info(f"received: {received}")

            pck = buoy.raw_package(pck)

            r = requests.post(ep, data=pck, headers={'SFY_AUTH_TOKEN' : target_token})
            logger.debug(f"status: {r.status_code}")

if __name__ == '__main__':
    migrate()
