import logging
import click
import yaml
from sfy.hub import Hub

logger = logging.getLogger(__name__)

@click.group()
def collection():
    pass

@collection.command()
@click.argument('config', type=click.File())
def archive(config):
    logger.info(f'Reading configuration file: {config.name}')

    hub = Hub.from_env()
    print(hub.buoys())

    b = hub.buoy('2022_CIRFA_JR_drifter_1')
    print(b)

    p = b.packages()
    print(p)

    j = b.json_package(p[0][0])
    print(j)

    # config = yaml.load(config)
    # print(config)

@collection.command()
@click.argument('config', type=click.File('w'))
def template(config):
    logger.info(f'Writing configuration file: {config.name}')

    # config = yaml.load(config)
    # print(config)
