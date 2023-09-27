from dataclasses import dataclass
import os
import json
import numpy as np
import base64
import sys
import logging
import pytz
import hashlib
import subprocess
from datetime import datetime, timedelta

from .timeseries import AxlTimeseries
from .event import Event

logger = logging.getLogger(__name__)

SENSORS_GRAVITY_STANDARD = 9.80665
SENSORS_DPS_TO_RADS = 0.017453293

def scale_u16_to_f32(mx, u):
    assert mx > 0.
    u16_max = np.iinfo(np.dtype(np.uint16)).max
    mx = np.float64(mx)
    v = np.float64(u)
    v = v * (2. * mx) / np.float64(u16_max)
    v = v - mx
    return np.float32(v)

class AxlCollection(AxlTimeseries):
    GAP_LIMIT = 10.  # limit in seconds before data is not considered continuous
    pcks: ['Axl']

    def __init__(self, pcks: ['Axl'], sorted_and_duplicates_removed=False):
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

        if not sorted_and_duplicates_removed:
            p = len(self.pcks)
            logger.debug("Removing duplicates..")
            self.pcks = list(frozenset(self.pcks))
            if p > len(self.pcks):
                logger.warning(f"Removed {p - len(self.pcks)} duplicates.")

            logger.debug('Sorting packages..')
            self.pcks.sort(key=lambda pck: pck.start)

    @staticmethod
    def from_storage_file(name, dev, file, raw=False):
        """
        Load all packages from a binary storage file (SD-card) into a collection of Axl packages.
        """
        logger.info(f"Parsing collection from {file} ({name} - {dev})..")
        assert os.path.exists(file), "file does not exist"
        if not raw:
            collection = subprocess.check_output(["sfypack", "--note", file])
        else:
            collection = subprocess.check_output(["sfypack", "--raw", "--note", file])
        collection = json.loads(collection)

        logger.info(f"Read {len(collection)} packages.")

        if len(collection) > 0:
            collection = [
                Axl.from_storage_json(name, dev, event) for event in collection
            ]
            return AxlCollection(collection)
        else:
            logger.error(f"No packages in {file}")
            return None

    def clip(self, start, end):
        """
        Clip the collection between start and end.
        """
        self.pcks = [
            pck for pck in self.pcks if pck.end >= start and pck.start <= end
        ]
        return self

    def segments(self, eps_gap=GAP_LIMIT):
        """
        Return iterable of collections split at gaps (above eps) in packages.
        """
        pcks = self.pcks.copy()
        segment = []

        while len(pcks) > 0:
            if len(segment) == 0:
                segment.append(pcks.pop(0))
            elif np.abs(segment[-1].end.timestamp() -
                        pcks[0].start.timestamp()) <= eps_gap:
                segment.append(pcks.pop(0))
            else:
                yield AxlCollection(segment, sorted_and_duplicates_removed=True)
                segment = []

        if len(segment) > 0:
            yield AxlCollection(segment, sorted_and_duplicates_removed=True)

    def samples(self):
        """
        Number of samples.
        """
        return sum(p.samples() for p in self.pcks)

    def max_gap(self):
        if len(self.pcks) == 1:
            return timedelta(seconds=0)

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
    def package_length(self):
        assert all((self.pcks[0].package_length == p.package_length for p in self.pcks)), "all packages must have the same length."
        return self.pcks[0].package_length

    @property
    def offsets(self):
        return np.concatenate([pck.offsets for pck in self.pcks])

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
    def has_raw(self):
        return all((pck.has_raw for pck in self.pcks))

    @property
    def ax(self):
        return np.concatenate([pck.ax for pck in self.pcks])

    @property
    def ay(self):
        return np.concatenate([pck.ay for pck in self.pcks])

    @property
    def az(self):
        return np.concatenate([pck.az for pck in self.pcks])

    @property
    def gx(self):
        return np.concatenate([pck.gx for pck in self.pcks])

    @property
    def gy(self):
        return np.concatenate([pck.gy for pck in self.pcks])

    @property
    def gz(self):
        return np.concatenate([pck.gz for pck in self.pcks])

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

    @property
    def start_times(self):
        return np.concatenate([pck.start_times for pck in self.pcks])

    def extra_attrs(self):
        return {
            'collection': 'yes',
            'max_gap': self.max_gap().total_seconds(),
            'max_gap:unit': 's',
            'number_of_packages': len(self.pcks)
        }


@dataclass(frozen=True)
class Axl(Event, AxlTimeseries):
    ## Payload and body
    length: int = None
    offset: int = None
    timestamp: int = None  # milliseconds, i64
    storage_id: int = None  # ID of package on SD card (if applicable), may not be unique.
    storage_version: int = None
    position_time: int = None  # seconds, time of location fix, u32
    temperature: float = None  # temperature measured by IMU
    lon: float = None
    lat: float = None
    freq: float = None
    accel_range: float = None # in [g]
    gyro_range: float = None  # in [dps]

    # Acceleration in m/s^2
    x: np.ndarray = None
    y: np.ndarray = None
    z: np.ndarray = None

    ax: np.ndarray = None
    ay: np.ndarray = None
    az: np.ndarray = None

    # Gyro in rad/s
    gx: np.ndarray = None
    gy: np.ndarray = None
    gz: np.ndarray = None

    from_store: bool = False

    # For testing purpuses
    __keep_payload__ = False

    def __eq__(self, o: 'Axl'):
        return self.duplicate(o)

    def __hash__(self):
        # Packages created with the default timestamp, no storage id, and without GPS location may cause a hash collision. Typically these packages are useless anyway, so we ignore the collision. The other fields are also not considered since they may be different if the same data is uploaded from the SD-card later.
        return hash((self.timestamp, self.storage_id, self.storage_version,
                     self.lon, self.lat, self.offset))

    def duplicate(self, o):
        if self.timestamp == o.timestamp and self.storage_id == o.storage_id and self.storage_version == o.storage_version and self.lon == o.lon and self.lat == o.lat and self.offset == o.offset:
            if np.array_equal(self.x, o.x) and np.array_equal(
                    self.y, o.y) and np.array_equal(self.z, o.z):
                return True

            logger.warn(
                f"duplicate timestamp {self.start}, but other fields mismatch."
            )
            return False

        else:
            return False

    @property
    def has_raw(self):
        return self.az is not None

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
        """
        Gets the time of the position acquired at the start of the acceleration data.
        """
        if self.position_time:
            return datetime.fromtimestamp(self.position_time, pytz.utc)
        else:
            return self.start

    @property
    def longitude(self):
        return self.lon

    @property
    def latitude(self):
        return self.lat

    @property
    def position_type(self):
        return 'gps'

    @property
    def start(self):
        """
        UTC Datetime of start of samples. Taking `offset` into account.
        """
        t = self.timestamp - (self.offset * 1000. / self.freq)
        t = datetime.fromtimestamp(t / 1000., tz=pytz.utc)
        return t

    @property
    def end(self):
        """
        UTC Datetime of start of samples. Taking `offset` into account.
        """
        t = self.timestamp + ((len(self.x) - 1 - self.offset) * 1000. / self.freq)
        t = datetime.fromtimestamp(t / 1000., tz=pytz.utc)
        return t

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
    def duration(self):
        """
        Duration in seconds.
        """
        return float(len(self.x)) / self.freq

    @property
    def mseconds(self):
        """
        Time vector in milliseconds (UTC). Taking `offset` into account.
        """
        t = (np.arange(0, len(self.x)) - self.offset) * 1000. / self.freq
        return self.timestamp + t

    @property
    def offsets(self):
        return np.array([self.offset])

    @property
    def package_length(self):
        return len(self.x)

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
    def start_times(self):
        return np.array([self.start])

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
    def parse(d) -> 'Axl':
        """
        Parse JSON string
        """

        data = json.loads(d)

        payload = data['payload']
        if not Axl.__keep_payload__:
            del data['payload']

        data['length'] = data['body']['length']
        data['offset'] = data['body'].get('offset', 0)
        data['timestamp'] = data['body'].get('timestamp', 0)
        data['storage_id'] = data['body'].get('storage_id', None)
        data['storage_version'] = data['body'].get('storage_version', 1)
        data['from_store'] = data['body'].get('from_store', False)
        data['lon'] = data['body'].get('lon')
        data['lat'] = data['body'].get('lat')
        data['position_time'] = data['body'].get('position_time')
        data['temperature'] = data['body'].get('temperature', 0.)
        data['freq'] = data['body'].get('freq', 208.)
        data['accel_range'] = data['body'].get('accel_range', 2.) # added in v6
        data['gyro_range'] = data['body'].get('gyro_range', 125. * 2.) # added in v6
        del data['body']

        # decode x, y, z
        payload = payload[:data['length']]
        payload = base64.b64decode(payload)

        ACCEL_MAX = SENSORS_GRAVITY_STANDARD * data['accel_range'] # [m/s^2]
        GYRO_MAX = SENSORS_DPS_TO_RADS * data['gyro_range'] # [rad/s]

        if data['storage_version'] < 5:
            payload = np.frombuffer(payload, dtype=np.float16)

            if sys.byteorder == 'big':
                logger.warning(
                    'host is big-endian, swapping bytes: this is not well-tested.'
                )
                payload.byteswap(inplace=True)

            x = payload[0::3]
            y = payload[1::3]
            z = payload[2::3]

        else:
            payload = np.frombuffer(payload, dtype=np.uint16)
            n = len(payload)

            if sys.byteorder == 'big':
                logger.warning(
                    'host is big-endian, swapping bytes: this is not well-tested.'
                )
                payload.byteswap(inplace=True)


            payload = scale_u16_to_f32(ACCEL_MAX, payload)

            assert len(payload) == n

            x = payload[0::3]
            y = payload[1::3]
            z = payload[2::3] + SENSORS_GRAVITY_STANDARD

        raw = data.pop('raw', None)

        if raw is not None:
            if sys.byteorder == 'big':
                logger.warning(
                    'host is big-endian, swapping bytes: this is not well-tested.'
                )
                raw.byteswap(inplace=True)

            gx = scale_u16_to_f32(GYRO_MAX, raw[0::6])
            gy = scale_u16_to_f32(GYRO_MAX, raw[1::6])
            gz = scale_u16_to_f32(GYRO_MAX, raw[2::6])
            ax = scale_u16_to_f32(ACCEL_MAX, raw[3::6])
            ay = scale_u16_to_f32(ACCEL_MAX, raw[4::6])
            az = scale_u16_to_f32(ACCEL_MAX, raw[5::6])

            assert len(ax) == len(x), 'Raw and filtered signal is not the same sample-rate, processing does not yet know how to handle that.'

            return Axl(**data, x=x, y=y, z=z, ax=ax, ay=ay, az=az, gx=gx, gy=gy, gz=gz)
        else:
            return Axl(**data, x=x, y=y, z=z)

    def json(self):
        data = self.__dict__.copy()
        body = {
            'length': self.length,
            'offset': self.offset,
            'timestamp': self.timestamp,
            'storage_id': self.storage_id,
            'storage_version': self.storage_version,
            'from_store': self.from_store,
            'position_time': self.position_time,
            'lon': self.lon,
            'lat': self.lat,
            'temperature': self.temperature,
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

    def samples(self):
        """
        Number of samples.
        """
        return len(self.z)

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
