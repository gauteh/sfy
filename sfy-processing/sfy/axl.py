from dataclasses import dataclass
import json
import numpy as np
import base64
import sys
import logging
import pytz
from datetime import datetime

logger = logging.getLogger(__name__)


@dataclass
class Axl:
    received: float
    routed: float

    event: str
    session: str
    product: str
    req: str
    file: str
    updates: int

    device: str
    sn: str

    tower_when: int
    tower_lon: float
    tower_lat: float
    tower_country: str
    tower_location: str
    tower_timezone: str
    tower_id: str

    project: dict

    ## Payload and body
    length: int
    offset: int
    timestamp: int # milliseconds, i64
    lon: float
    lat: float
    freq: float

    x: np.ndarray
    y: np.ndarray
    z: np.ndarray

    when: int = None

    where_olc: float = None
    where_lat: float = None
    where_lon: float = None
    where_location: str = None
    where_country: str = None
    where_timezone: str = None

    @property
    def start(self):
        """
        UTC Datetime of start of samples. Taking `offset` into account.
        """
        return datetime.fromtimestamp(self.timestamp / 1000. - (self.offset / self.freq), pytz.utc)

    @property
    def received_dt(self):
        return datetime.fromtimestamp(self.received, pytz.utc)

    @property
    def duration(self):
        """
        Duration in seconds.
        """
        return len(self.x) / self.freq


    @property
    def mseconds(self):
        """
        Time vector in milliseconds (UTC).
        """
        t = np.arange(0, len(self.x)) / self.freq
        return self.timestamp + t

    def __repr__(self):
        return f"[Axl received={self.received} t={self.start} -> {'%.2f' % self.duration}s sz={len(self.x)}x3 @ f={self.freq}Hz, lon={self.lon}E lat={self.lat}N]"

    @staticmethod
    def parse(d) -> 'Axl':
        """
        Parse JSON string
        """

        data = json.loads(d)

        payload = data['payload']
        del data['payload']

        data['length'] = data['body']['length']
        data['offset'] = data['body'].get('offset', 0)
        data['timestamp'] = data['body']['timestamp']
        data['lon'] = data['body'].get('lon')
        data['lat'] = data['body'].get('lat')
        data['freq'] = data['body'].get('freq', 208.)
        del data['body']

        # decode x, y, z
        payload = payload[:data['length']]
        payload = base64.b64decode(payload)
        payload = np.frombuffer(payload, dtype=np.float16)

        if sys.byteorder == 'big':
            logger.warning(
                'host is big-endian, swapping bytes: this is not well-tested.')
            payload.byteswap(inplace=True)

        x = payload[0::3]
        y = payload[1::3]
        z = payload[2::3]

        return Axl(**data, x=x, y=y, z=z)

    @staticmethod
    def from_file(path) -> 'Axl':
        with open(path, 'r') as fd:
            return Axl.parse(fd.read())

