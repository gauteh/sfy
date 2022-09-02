import json
import math
from dataclasses import dataclass
from datetime import datetime
from typing import Any
import pytz
import logging

logger = logging.getLogger(__name__)

@dataclass(frozen=True)
class Event:
    received: float
    event: str
    file: str

    device: str
    sn: str

    tower_when: int
    tower_timezone: str
    tower_lon: float = None
    tower_lat: float = None

    # These are either new or deprecated, only present in some packages.
    best_id: str = None
    best_location_type: str = None
    best_location_when: float = None
    best_location: str = None
    best_country: str = None
    best_timezone: str = None
    best_lat: float = None
    best_lon: float = None

    routed: float = None
    session: str = None
    product: str = None
    req: str = None
    updates: int = None

    project: dict = None

    tower_country: str = None
    tower_location: str = None
    tower_id: str = None

    when: int = None

    where_when: int = None
    where_olc: float = None
    where_lat: float = None
    where_lon: float = None
    where_location: str = None
    where_country: str = None
    where_timezone: str = None

    body: Any = None
    payload: Any = None

    @property
    def longitude(self):
        return self.best_lon

    @property
    def latitude(self):
        return self.best_lat

    @property
    def best_position_time(self):
        return datetime.fromtimestamp(self.best_location_when, pytz.utc)

    @property
    def position_type(self):
        """
        Returns whether the position is determined from gps or from the cell tower.

        Values: 'gps' or 'tower'.
        """
        return self.best_location_type

    @property
    def fname(self) -> str:
        return f'{math.floor(self.received * 1000.)}-{self.event}_{self.file}.json'

    @property
    def received_datetime(self):
        """
        UTC Datetime of time received or uploaded from notecard.
        """
        return datetime.fromtimestamp(self.received, pytz.utc)

    @property
    def added_datetime(self):
        """
        UTC Datetime of time added to notecard.
        """
        return datetime.fromtimestamp(self.when, pytz.utc) if self.when else None

    def save(self, path):
        data = self.json()
        with open(path, 'w') as fd:
            fd.write(data)

    @classmethod
    def try_parse(cls, d):
        try:
            return cls.parse(d)
        except (KeyError, json.decoder.JSONDecodeError) as e:
            # logger.exception(e)
            logger.error(f"failed to parse file: {d}: {e}")
            return None

    @staticmethod
    def parse(d):
        """
        Parse JSON string
        """

        data = json.loads(d)

        return Event(**data)

    def json(self):
        data = self.__dict__.copy()

        return json.dumps(data)
