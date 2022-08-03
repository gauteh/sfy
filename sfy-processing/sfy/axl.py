from dataclasses import dataclass
import json
import math
import os
import numpy as np
import base64
import sys
import logging
import pytz
import hashlib
import subprocess
from datetime import datetime

from .timeseries import AxlTimeseries

logger = logging.getLogger(__name__)


class AxlCollection(AxlTimeseries):
    GAP_LIMIT = 10.  # limit in seconds before data is not considered continuous

    pcks: ['Axl']

    def __init__(self, pcks: ['Axl'], duplicates_removed=False):
        assert len(pcks) > 0, "must be at least one package"

        assert all(pck.frequency == pcks[0].frequency
                   for pck in pcks), "all packages must be the same frequency"

        self.pcks = pcks.copy()

        # Remove duplicates:
        #
        # Duplicates can occur when the buoy tries to send a data-package, and
        # the notecard / modem indicates an error (or there was an I2C error),
        # while the package still went through. The buoy will then try to send
        # the data once more. All the fields of the package (body, timestamp,
        # _and_ data vectors) will be _identical_. We can therefore reliably
        # identify these packages.
        #

        if not duplicates_removed:
            p = len(self.pcks)
            logger.debug("Removing duplicates..")
            self.pcks = list(frozenset(self.pcks))
            if p > len(self.pcks):
                logger.warning(f"Removed {p - len(self.pcks)} duplicates.")

        self.pcks.sort(key=lambda pck: pck.start)

    @staticmethod
    def from_storage_file(name, dev, file):
        """
        Load all packages from a binary storage file (SD-card) into a collection of Axl packages.
        """
        logger.info(f"Parsing collection from {file} ({name} - {dev})..")
        assert os.path.exists(file), "file does not exist"
        collection = subprocess.check_output(["sfypack", "--note", file])
        collection = json.loads(collection)
        logger.info(f"Read {len(collection)} packages.")

        collection = [
            Axl.from_storage_json(name, dev, event) for event in collection
        ]
        return AxlCollection(collection)

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
                yield AxlCollection(segment, duplicates_removed=True)
                segment = []

        if len(segment) > 0:
            yield AxlCollection(segment, duplicates_removed=True)

    def max_gap(self):
        if len(self.pcks) == 1:
            return np.nan

        d = []
        for i, _s in enumerate(self.pcks[:-1]):
            d.append(self.pcks[i + 1].start - self.pcks[i].end)
        return max(d)

    def __add__(self, other):
        return AxlCollection(self.pcks + other.pcks)

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
    def position_times(self):
        return np.concatenate([pck.position_times for pck in self.pcks])

    @property
    def lons(self):
        return [pck.lon for pck in self.pcks]

    @property
    def lats(self):
        return [pck.lat for pck in self.pcks]

    @property
    def device(self):
        return self.pcks[0].device

    @property
    def sn(self):
        return self.pcks[0].sn

    @property
    def storage_ids(self):
        return np.concatenate([pck.storage_ids for pck in self.pcks])

    @property
    def received_times(self):
        return np.concatenate([pck.received_times for pck in self.pcks])

    @property
    def added_times(self):
        return np.concatenate([pck.added_times for pck in self.pcks])

    def extra_attrs(self):
        return {
            'collection': 'yes',
            'max_gap': self.max_gap().total_seconds(),
            'max_gap:unit': 's',
            'number_of_packages': len(self.pcks)
        }


@dataclass(frozen=True)
class Axl(AxlTimeseries):
    received: float
    event: str
    file: str

    device: str
    sn: str

    tower_when: int
    tower_lon: float
    tower_lat: float
    tower_timezone: str

    ## Payload and body
    length: int
    offset: int
    timestamp: int  # milliseconds, i64
    storage_id: int  # ID of package on SD card (if applicable), may not be unique.
    position_time: int  # time of location fix, u32
    lon: float
    lat: float
    freq: float

    x: np.ndarray
    y: np.ndarray
    z: np.ndarray

    # These are either new or deprecated, only present in some packages.
    best_id: str = None
    best_location_type: str = None
    best_location_when: str = None
    best_location: str = None
    best_country: str = None
    best_timezone: str = None
    best_lat: str = None
    best_lon: str = None

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
    from_store: bool = False

    def __eq__(self, o: 'Axl'):
        return self.duplicate(o)

    def __hash__(self):
        # Packages created with the default timestamp, no storage id, and without GPS location may cause a hash collision. Typically these packages are useless anyway, so we ignore the collision. The other fields are also not considered since they may be different if the same data is uploaded from the SD-card later.
        return hash((self.timestamp, self.storage_id, self.lon, self.lat,
                     self.offset, self.storage_id))

    def duplicate(self, o):
        if self.timestamp == o.timestamp and self.storage_id == o.storage_id and self.lon == o.lon and self.lat == o.lat and self.offset == o.offset:
            if all(self.x == o.x) and all(self.y == o.y) and all(
                    self.z == o.z):
                return True

            logger.warn(
                f"duplicate timestamp {self.start}, but other fields mismatch."
            )
            return False

        else:
            return False

    @property
    def fname(self) -> str:
        return f'{math.floor(self.received * 1000.)}-{self.event}_{self.file}.json'

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
    def best_position_time(self):
        if self.position_time:
            try:
                return datetime.fromtimestamp(self.position_time, pytz.utc)
            except ValueError as ex:
                return datetime.fromtimestamp(self.position_time / 1000.,
                                              pytz.utc)
        else:
            return self.start

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

    @property
    def position_times(self):
        return np.array([self.position_time])

    @property
    def received_times(self):
        return np.array([self.received_datetime])

    @property
    def added_times(self):
        return np.array([self.added_datetime])

    @property
    def storage_ids(self):
        return np.array([self.storage_id])

    @property
    def lons(self):
        return [self.lon]

    @property
    def lats(self):
        return [self.lat]

    def __repr__(self):
        return f"[Axl received={self.received} storage_id={self.storage_id} t={self.start} -> {'%.2f' % self.duration}s sz={len(self.x)}x3 @ f={self.freq}Hz, lon={self.lon}E lat={self.lat}N]"

    @staticmethod
    def try_parse(d) -> 'Axl':
        try:
            return Axl.parse(d)
        except (KeyError, json.decoder.JSONDecodeError) as e:
            # logger.exception(e)
            logger.error(f"failed to parse file: {d}: {e}")
            return None

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
        data['storage_id'] = data['body'].get('storage_id', None)
        data['position_time'] = data['body'].get('position_time',
                                                 data['body']['timestamp'])
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
            'storage_id': self.storage_id,
            'position_time': self.position_time,
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

    @staticmethod
    def from_storage_json(name, dev, event):
        """
        Parse an Axl dictionary obtained from a binary storage file (from SD card).
        """
        time, lat, lon = event['body']['timestamp'], event['body'][
            'lat'], event['body']['lon']
        time_s = time / 1.e3

        event['device'] = "dev:" + dev[3:]
        event['sn'] = name
        event['received'] = time_s
        event['when'] = int(time_s)
        event['from_store'] = True
        event['file'] = "axl.qo"
        event['where_when'] = int(time_s)
        event['where_lat'] = lat
        event['where_lon'] = lon
        event['where_timezone'] = 'UTC'
        event['tower_when'] = int(time_s)
        event['tower_lat'] = lat
        event['tower_lon'] = lon
        event['tower_timezone'] = 'UTC'

        # simulate blues notecard JSON where default value is removed
        for key in ['lon', 'lat', 'timestamp', 'position_time']:
            if event['body'].get(key) == 0:
                del event['body'][key]

        # make event id
        hash = hashlib.shake_256()
        hash.update(str(event['body']).encode('utf-8'))
        hash.update(event['payload'].encode('utf-8'))
        hash = hash.hexdigest(length=int((36 - 4) / 2))
        hash = f"{hash[:8]}-{hash[8:12]}-{hash[12:16]}-{hash[16:20]}-{hash[20:]}"
        event['event'] = hash

        # uri = f"{int(time):013d}-{event['event']}_axl.qo.json"

        return Axl.parse(json.dumps(event))
