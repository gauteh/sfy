#! /usr/bin/env python

import click
import matplotlib.pyplot as plt
from datetime import timedelta, datetime
from tabulate import tabulate
import numpy as np
import time
import os

import sfy
from sfy.hub import Hub
from sfy.axl import Axl, AxlCollection
from sfy import signal
from sfy.timeutil import utcify
import trajan
import logging

logger = logging.getLogger(__name__)


@click.group()
def omb():
    pass


@omb.command(name='archive', help='Archive OMB buoy')
@click.argument('dev')
@click.option('--start',
              default=None,
              help='Filter packages sent after this time',
              type=click.DateTime())
@click.option('--end',
              default=None,
              help='Filter packages sent before this time',
              type=click.DateTime())
def archive(dev, start, end):
    hub = Hub.from_env()
    buoy = hub.buoy(dev)
    logger.info(f"Listing packages for {buoy}")
    pcks = buoy.packages_range(start, end)
    print(pcks)
