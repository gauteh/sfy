import base64
import json
import math
from dataclasses import dataclass
from datetime import datetime
from typing import Any
import pytz
import logging

logger = logging.getLogger(__name__)

from .hub import Buoy


class OmbBuoy(Buoy):

    def __init__(self, hub, dev, name, buoy_type, last):
        self.buoy_type = 'omb'
        assert buoy_type == self.buoy_type

        self.hub = hub
        self.dev = dev
        self.name = name
        if len(last) > 0:
            self.__last__ = OmbEvent(**json.loads(base64.b64decode(last)))
        else:
            self.__last__ = None


@dataclass(frozen=True)
class OmbEvent:
    account: str
    datetime: int
    type: str

    device: str

    body: Any = None
    payload: Any = None

    # received_datetime: int = 0
    version: int = 0

    @property
    def received_datetime(self):
        """
        UTC Datetime of time received or uploaded from notecard.
        """
        return datetime.fromtimestamp(self.datetime / 1000., pytz.utc)

    @property
    def received(self):
        return self.datetime / 1000.
