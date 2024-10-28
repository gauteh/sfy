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

from .timeseries import EgpsTimeseries
from .event import Event
from .axl import scale_u16_to_f32

logger = logging.getLogger(__name__)


class EgpsCollection(EgpsTimeseries):
    GAP_LIMIT = 10.  # limit in seconds before data is not considered continuous
    pcks: ['Egps']

    def __init__(self, pcks: ['Egps'], sorted_and_duplicates_removed=False):
        assert len(pcks) > 0, "must be at least one package"

        # assert all(pck.frequency == pcks[0].frequency
        #            for pck in pcks), "all packages must be the same frequency"

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
                yield EgpsCollection(segment,
                                     sorted_and_duplicates_removed=True)
                segment = []

        if len(segment) > 0:
            yield EgpsCollection(segment, sorted_and_duplicates_removed=True)

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
        return EgpsCollection(self.pcks + other.pcks)

    def __len__(self):
        return len(self.pcks)

    @property
    def duration(self):
        return sum(pck.duration for pck in self.pcks)

    @property
    def package_length(self):
        return [pck.package_length for pck in self.pcks]

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
    def n(self):
        return np.concatenate([pck.n for pck in self.pcks])

    @property
    def e(self):
        return np.concatenate([pck.e for pck in self.pcks])

    @property
    def z(self):
        return np.concatenate([pck.z for pck in self.pcks])

    @property
    def vn(self):
        return np.concatenate([pck.vn for pck in self.pcks])

    @property
    def ve(self):
        return np.concatenate([pck.ve for pck in self.pcks])

    @property
    def vz(self):
        return np.concatenate([pck.vz for pck in self.pcks])

    @property
    def position_times(self):
        return np.concatenate([pck.position_times for pck in self.pcks])

    @property
    def lons(self):
        return [pck.longitude for pck in self.pcks]

    @property
    def lats(self):
        return [pck.latitude for pck in self.pcks]

    @property
    def device(self):
        return self.pcks[0].device

    @property
    def sn(self):
        return self.pcks[0].sn

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
        attrs = {
            'collection': 'yes',
            'max_gap': self.max_gap().total_seconds(),
            'max_gap:unit': 's',
            'number_of_packages': len(self.pcks)
        }

        lonlat_range = [pck.lonlat_range for pck in self.pcks]
        msl_range = [pck.msl_range for pck in self.pcks]
        vel_range = [pck.vel_range for pck in self.pcks]

        attrs['lonlat_range'] = lonlat_range[0]
        attrs['lonlat_range:unit'] = 'deg * 1e7'

        attrs['msl_range'] = msl_range[0]
        attrs['msl_range:unit'] = 'mm'

        attrs['vel_range'] = vel_range[0]
        attrs['vel_range:unit'] = 'mm/s'

        return attrs


@dataclass(frozen=True)
class Egps(Event):
    ## Payload and body
    length: int = None
    timestamp: int = None  # milliseconds, i64
    version: int = None
    freq: float = None
    lon: float = None
    lat: float = None
    msl: float = None
    freq: float = None
    lonlat_range: float = None  # in [deg * 1e7]
    msl_range: float = None  # in [mm]
    vel_range: float = None  # in [mm/s]

    # Position [m]
    n: np.ndarray = None
    e: np.ndarray = None
    z: np.ndarray = None

    # Velocity [m/s]
    vn: np.ndarray = None
    ve: np.ndarray = None
    vz: np.ndarray = None

    # For testing purpuses
    __keep_payload__ = False

    def __eq__(self, o: 'Egps'):
        return self.duplicate(o)

    def __hash__(self):
        return hash(
            (self.timestamp, self.version, self.lon, self.lat, self.msl))

    def duplicate(self, o):
        if self.timestamp == o.timestamp and self.version == o.version and self.lon == o.lon and self.lat == o.lat and self.msl == o.msl:
            if np.array_equal(self.n, o.n) and np.array_equal(
                    self.e, o.e) and np.array_equal(self.z, o.z):
                return True

            logger.warn(
                f"duplicate timestamp {self.start}, but other fields mismatch."
            )
            return False

        else:
            return False

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
        if self.timestamp:
            return datetime.fromtimestamp(self.timestamp / 1000., pytz.utc)
        else:
            return self.start

    @property
    def longitude(self):
        return self.lon / 1.e7

    @property
    def latitude(self):
        return self.lat / 1.e7

    @property
    def position_type(self):
        return 'gps'

    @property
    def start(self):
        """
        UTC Datetime of start of samples.
        """
        return datetime.fromtimestamp(self.timestamp / 1000., tz=pytz.utc)

    @property
    def end(self):
        """
        UTC Datetime of end of samples.
        """
        return datetime.fromtimestamp(
            (self.timestamp + 1000. / self.freq * len(self.n)) / 1000.,
            tz=pytz.utc)

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
        return float(len(self.n)) / self.freq

    @property
    def mseconds(self):
        """
        Time vector in milliseconds (UTC). Taking `offset` into account.
        """
        t = (np.arange(0, len(self.n))) * 1000. / self.freq
        return self.timestamp + t

    @property
    def package_length(self):
        return len(self.n)

    @property
    def position_times(self):
        return np.array([self.timestamp / 1000.])

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
    def lons(self):
        return [self.lon]

    @property
    def lats(self):
        return [self.lat]

    def __repr__(self):
        return f"[Egps received={self.received} t={self.start} -> {'%.2f' % self.duration}s sz={len(self.n)}x6 @ f={self.freq}Hz, lon={self.longitude}E lat={self.latitude}N]"

    @staticmethod
    def parse(d) -> 'Egps':
        """
        Parse JSON string
        """

        data = json.loads(d)

        payload = data['payload']
        if not Egps.__keep_payload__:
            del data['payload']

        data['length'] = data['body']['length']
        data['timestamp'] = data['body']['timestamp']
        data['version'] = data['body']['version']
        data['lon'] = data['body']['lon']
        data['lat'] = data['body']['lat']
        data['msl'] = data['body']['msl']
        data['freq'] = data['body']['freq']
        data['lonlat_range'] = data['body']['lonlat_range']
        data['msl_range'] = data['body']['msl_range']
        data['vel_range'] = data['body'].get('vel_range',
                                             200.0 * 1.0e6 / 60. / 60)
        del data['body']

        # decode x, y, z
        payload = payload[:data['length']]
        payload = base64.b64decode(payload)

        if (len(payload) % (2 * 3)) != 0:
            raise ValueError(
                f"length of payload: {len(payload)}, does not match expected number of values"
            )
        payload = np.frombuffer(payload, dtype=np.uint16)
        N = len(payload)

        if sys.byteorder == 'big':
            logger.warning(
                'host is big-endian, swapping bytes: this is not well-tested.')
            payload.byteswap(inplace=True)

        n = scale_u16_to_f32(data['lonlat_range'], payload[0::6]) + data['lat']
        e = scale_u16_to_f32(data['lonlat_range'], payload[1::6]) + data['lon']
        z = scale_u16_to_f32(data['msl_range'], payload[2::6]) + data['msl']
        vn = scale_u16_to_f32(data['vel_range'], payload[3::6])
        ve = scale_u16_to_f32(data['vel_range'], payload[4::6])
        vz = scale_u16_to_f32(data['vel_range'], payload[5::6])

        assert len(n) == N / 6

        return Egps(**data, n=n, e=e, z=z, vn=vn, ve=ve, vz=vz)

    def samples(self):
        """
        Number of samples.
        """
        return len(self.z)

    @staticmethod
    def from_file(path) -> 'Egps':
        with open(path, 'r') as fd:
            return Egps.parse(fd.read())
