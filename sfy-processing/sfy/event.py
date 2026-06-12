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
    sn: str = None

    tower_when: int = None
    tower_timezone: str = None
    app: str = None
    note: str = None
    deleted: any = None
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

    fleets: str = None
    platform: Any = None
    tls: bool = None
    firmware_notecard: str = None

    body: Any = None
    payload: Any = None
    payload_length: Any = None
    transport: Any = None
    batch_received: Any = None
    batch_number: Any = None
    batch_total: Any = None

    @property
    def longitude(self):
        return self.best_lon

    @property
    def latitude(self):
        return self.best_lat

    @property
    def best_position_time(self):
        return datetime.fromtimestamp(self.best_location_when, pytz.utc) if self.best_location_when else None

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

    def get_voltage(self):
        return self.body.get('voltage', None) if self.body else None

    def get_temp(self):
        return self.body.get('temperature', None) if self.body else None

    @classmethod
    def try_parse(cls, d):
        try:
            return cls.parse(d)
        except (KeyError, json.decoder.JSONDecodeError, ValueError) as e:
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


@dataclass(frozen=True)
class Session(Event):
    """
    Parsed _session.qo event from sfy4 buoys. These carry modem / radio
    statistics and device state as top-level fields (not inside body).
    """
    # Modem / radio fields
    sku: str = None
    ordering_code: str = None
    bearer: str = None
    cellid: str = None
    iccid: str = None
    apn: str = None
    rssi: float = None
    sinr: float = None
    rsrp: float = None
    rsrq: float = None
    rat: str = None
    bars: int = None

    # Device state
    voltage: float = None
    temp: float = None
    moved: int = None
    orientation: str = None

    # Hub statistics (present in closing session.end events)
    hub_last_work_done: int = None
    hub_duration_secs: int = None
    hub_events_routed: int = None
    hub_rcvd_bytes: int = None
    hub_sent_bytes: int = None
    hub_tls_sessions: int = None
    hub_tcp_sessions: int = None
    hub_sent_notes: int = None

    @staticmethod
    def parse(d) -> 'Session':
        """
        Parse a _session.qo JSON string.
        """
        data = json.loads(d)
        return Session(**data)

    @classmethod
    def try_parse(cls, d):
        try:
            return cls.parse(d)
        except (KeyError, json.decoder.JSONDecodeError, ValueError, TypeError) as e:
            logger.error(f"failed to parse session file: {e}")
            return None

    def get_voltage(self):
        return self.voltage

    def get_temp(self):
        return self.temp
