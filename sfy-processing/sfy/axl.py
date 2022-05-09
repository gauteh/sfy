from dataclasses import dataclass
import json
import numpy as np
import base64
import sys
import logging
import pytz
from datetime import datetime, timedelta

from .timeseries import AxlTimeseries

logger = logging.getLogger(__name__)


class AxlCollection(AxlTimeseries):
    GAP_LIMIT = 1.  # limit in seconds before data is not considered continuous

    pcks: ['Axl']

    def __init__(self, pcks):
        assert len(pcks) > 0, "must be at least one package"

        self.pcks = pcks
        self.pcks.sort(key=lambda pck: pck.start)

        assert all(pck.frequency == pcks[0].frequency
                   for pck in pcks), "all packages must be the same frequency"

    def clip(self, start, end):
        """
        Clip the collection between start and end.
        """
        self.pcks = [
            pck for pck in self.pcks if pck.start >= start and pck.start <= end
        ]

    def segments(self, eps_gap=GAP_LIMIT):
        """
        Return iterable of collections split at gaps (above eps) in packages.
        """
        pcks = self.pcks.copy()
        segment = []

        assert len(pcks) > 0

        while len(pcks) > 0:
            if len(segment) == 0:
                segment.append(pcks.pop(0))
            elif np.abs(segment[-1].end.timestamp() -
                        pcks[0].start.timestamp()) <= eps_gap:
                segment.append(pcks.pop(0))
            else:
                yield AxlCollection(segment)
                segment = []

        if len(segment) > 0:
            yield AxlCollection(segment)

    def __len__(self):
        return len(self.pcks)

    @property
    def duration(self):
        return sum(pck.duration for pck in self.pcks)

    @property
    def start(self):
        return self.pcks[0].start

    @property
    def end(self):
        return self.pcks[-1].end

    @property
    def frequency(self):
        return self.pcks[0].frequency

    @property
    def dt(self):
        return self.pcks[0].dt

    @property
    def time(self):
        return np.concatenate([pck.time for pck in self.pcks])

    @property
    def mseconds(self):
        return np.concatenate([pck.mseconds for pck in self.pcks])

    @property
    def x(self):
        return np.concatenate([pck.x for pck in self.pcks])

    @property
    def y(self):
        return np.concatenate([pck.y for pck in self.pcks])

    @property
    def z(self):
        return np.concatenate([pck.z for pck in self.pcks])

    @property
    def lons(self):
        return [pck.lon for pck in self.pcks]

    @property
    def lats(self):
        return [pck.lat for pck in self.pcks]


@dataclass(frozen=True)
class Axl(AxlTimeseries):
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
    timestamp: int  # milliseconds, i64
    lon: float
    lat: float
    freq: float

    x: np.ndarray
    y: np.ndarray
    z: np.ndarray

    # I think these are deprecated, and only present in old events.
    best_id: str = None
    best_location_type: str = None
    best_location_when: str = None
    best_location: str = None
    best_country: str = None
    best_timezone: str = None
    best_lat: str = None
    best_lon: str = None

    when: int = None

    where_when: int = None
    where_olc: float = None
    where_lat: float = None
    where_lon: float = None
    where_location: str = None
    where_country: str = None
    where_timezone: str = None

    def __eq__(self, o: 'Axl'):
        eq = self.__dict__.keys() == o.__dict__.keys()
        if eq:
            for k in self.__dict__.keys():
                if isinstance(self.__dict__[k], np.ndarray):
                    eq &= all(self.__dict__[k] == o.__dict__[k])
                else:
                    eq &= self.__dict__[k] == o.__dict__[k]

                if not eq:
                    return False
        else:
            return False

        return True

    @property
    def dt(self) -> float:
        """
        Sample rate
        """
        return 1. / self.freq

    @property
    def frequency(self):
        return self.freq

    @property
    def start(self):
        """
        UTC Datetime of start of samples. Taking `offset` into account.
        """
        return datetime.fromtimestamp(
            self.timestamp / 1000. - (self.offset / self.freq), pytz.utc)

    @property
    def end(self):
        """
        UTC Datetime of start of samples. Taking `offset` into account.
        """
        return datetime.fromtimestamp(
            self.timestamp / 1000. - (self.offset / self.freq) + self.duration,
            pytz.utc)

    @property
    def time(self):
        """
        UTC datetime timestamps of samples
        """
        t = np.array([
            datetime.fromtimestamp(s / 1000., tz=pytz.utc)
            for s in self.mseconds
        ])
        return t

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
        return datetime.fromtimestamp(self.when, pytz.utc)

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
        t = np.arange(0, len(self.x)) * 1000. / self.freq
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

    def json(self):
        data = self.__dict__.copy()
        body = {
            'length': self.length,
            'offset': self.offset,
            'timestamp': self.timestamp,
            'lon': self.lon,
            'lat': self.lat,
            'freq': self.freq,
        }

        payload = np.zeros((len(self.x) * 3, ), dtype=np.float16)
        payload[0::3] = self.x
        payload[1::3] = self.y
        payload[2::3] = self.z

        if sys.byteorder == 'big':
            logger.warning(
                'host is big-endian, swapping bytes: this is not well-tested.')
            payload.byteswap(inplace=True)

        payload = base64.b64encode(payload.tobytes()).decode()

        del data['length'], data['offset'], data['timestamp'], data[
            'lon'], data['lat'], data['freq']
        del data['x'], data['y'], data['z']

        data['payload'] = payload
        data['body'] = body

        return json.dumps(data)

    def save(self, path):
        data = self.json()
        with open(path, 'w') as fd:
            fd.write(data)

    @staticmethod
    def from_file(path) -> 'Axl':
        with open(path, 'r') as fd:
            return Axl.parse(fd.read())
