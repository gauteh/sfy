import os
from pathlib import Path
from urllib.parse import urljoin
import requests
from datetime import datetime
import pytz
import logging
logger = logging.getLogger(__name__)

from .axl import Axl


class Hub:
    endpoint: str
    key: str
    cache: Path

    def __init__(self, endpoint, key, cache):
        """
        Set up a Hub client.

            endpoint: URL to sfy hub.

            key: Read token.
        """
        self.endpoint = endpoint

        if self.endpoint[-1] != '/':
            self.endpoint += '/'

        self.key = key
        self.cache = Path(cache)

        if not self.cache.exists():
            os.makedirs(self.cache, exist_ok=True)

    @staticmethod
    def from_env():
        from urllib.parse import urljoin

        API = os.getenv('SFY_SERVER')
        KEY = os.getenv('SFY_READ_TOKEN')
        CACHE = os.getenv('SFY_DATA_CACHE')

        if API is None or KEY is None or CACHE is None:
            raise Exception("No API, KEY or CACHE")

        API = urljoin(API, 'buoys')
        return Hub(urljoin(API, 'buoys'), KEY, CACHE)

    def __request__(self, path):
        url = urljoin(self.endpoint, path)

        r = requests.get(url, headers={'SFY_AUTH_TOKEN': self.key})
        r.raise_for_status()

        return r

    def __json_request__(self, path):
        return self.__request__(path).json()

    def buoys(self):
        """
        Get list of buoys.
        """
        return [Buoy(self, d) for d in self.__json_request__('./')]

    def buoy(self, dev):
        return next(filter(lambda b: dev in b.dev, self.buoys()))


class Buoy:
    hub: Hub
    dev: str

    def __init__(self, hub, dev):
        self.hub = hub
        self.dev = dev

    def __repr__(self):
        return f"Buoy <{self.dev}>"

    def packages(self):
        return self.hub.__json_request__(self.dev)

    def raw_package(self, pck):
        return self.hub.__json_request__(f'{self.dev}/{pck}')

    def packages_range(self, start=None, end=None):
        """
        Get packages _uploaded_ between start and end datetimes. This is not necessarily the timespan the packages cover.
        """
        pcks = self.packages()

        pcks = ((pck.split('-')[0], pck) for pck in pcks)
        pcks = ((datetime.fromtimestamp(float(pck[0]) / 1000.,
                                        pytz.utc), pck[1]) for pck in pcks)

        if start is not None:
            if start.tzinfo is None:
                start = pytz.utc.localize(start)
            pcks = filter(lambda pck: pck[0] >= start, pcks)

        if end is not None:
            if end.tzinfo is None:
                end = pytz.utc.localize(end)
            pcks = filter(lambda pck: pck[0] <= end, pcks)

        return list(pcks)

    def package(self, pck):
        dev_path = self.hub.cache / self.dev
        os.makedirs(dev_path, exist_ok=True)

        pckf: Path = dev_path / pck
        if not pckf.exists():
            try:
                with open(pckf, 'w') as fd:
                    fd.write(self.hub.__request__(f'{self.dev}/{pck}').text)
            except:
                os.remove(pckf)

        try:
            return Axl.from_file(pckf)
        except:
            logger.error(f"failed to parse file: {self.dev/pckf}")
            return None
