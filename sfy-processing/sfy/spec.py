from dataclasses import dataclass
import json
import numpy as np
import base64
import sys
import logging
import pytz
from datetime import datetime, timedelta

from .timeseries import SpecTimeseries
from .event import Event
from .axl import scale_u16_to_f32

logger = logging.getLogger(__name__)


class SpecCollection(SpecTimeseries):
    GAP_LIMIT = 10.  # limit in seconds before data is not considered continuous
    pcks: ['Spec']

    def __init__(self, pcks: ['Spec'], sorted_and_duplicates_removed=False):
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
                yield SpecCollection(segment,
                                     sorted_and_duplicates_removed=True)
                segment = []

        if len(segment) > 0:
            yield SpecCollection(segment, sorted_and_duplicates_removed=True)

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
        return SpecCollection(self.pcks + other.pcks)

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
class Spec(Event):
    ## Payload and body
    timestamp: int = None  # milliseconds, i64
    max: float = None
    lon: float = None
    lat: float = None
    time: int = None
    ltime: int = None

    # Position [m]
    E: np.ndarray = None

    # For testing purpuses
    __keep_payload__ = False

    def __eq__(self, o: 'Spec'):
        return self.duplicate(o)

    def __hash__(self):
        return hash(
            (self.timestamp, self.lon, self.lat, self.max))

    def duplicate(self, o):
        if self.timestamp == o.timestamp and self.lon == o.lon and self.lat == o.lat and self.max == o.max:
            if np.array_equal(self.E, o.E):
                return True

            logger.warn(
                f"duplicate timestamp {self.start}, but other fields mismatch."
            )
            return False

        else:
            return False


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
    def package_length(self):
        return len(self.E)

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
        return f"[Spec received={self.received} t={self.start} -> {'%.2f' % self.duration}s sz={len(self.E)}, lon={self.longitude}E lat={self.latitude}N]"

    @staticmethod
    def parse(d) -> 'Spec':
        """
        Parse JSON string
        """

        data = json.loads(d)

        payload = data['payload']
        if not Spec.__keep_payload__:
            del data['payload']

        data['length'] = data['body']['length']
        data['timestamp'] = data['body']['timestamp']
        data['version'] = data['body']['version']
        data['max'] = data['body']['max']
        del data['body']

        # decode E
        # payload = payload[:data['length']]
        payload = base64.b64decode(payload)

        # if (len(payload) % (2 * 3)) != 0:
        #     raise ValueError(
        #         f"length of payload: {len(payload)}, does not match expected number of values"
        # )
        payload = np.frombuffer(payload, dtype=np.uint16)
        N = len(payload)

        if sys.byteorder == 'big':
            logger.warning(
                'host is big-endian, swapping bytes: this is not well-tested.')
            payload.byteswap(inplace=True)

        def scale_u16_to_f32_positive(mx, u):
            assert mx > 0.
            u16_max = np.iinfo(np.dtype(np.uint16)).max
            mx = np.float64(mx)
            v = np.float64(u)
            v = v * mx / np.float64(u16_max)
            v = v
            return np.float32(v)

        E = scale_u16_to_f32_positive(data['max'], payload)

        return Spec(**data, E=E)

    def samples(self):
        """
        Number of samples.
        """
        return len(self.E)

    @staticmethod
    def from_file(path) -> 'Spec':
        with open(path, 'r') as fd:
            return Spec.parse(fd.read())

