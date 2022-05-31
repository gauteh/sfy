import os
from pathlib import Path
from urllib.parse import urljoin
import requests
from datetime import datetime, timezone
import logging
from tqdm import tqdm
import json
import math

logger = logging.getLogger(__name__)

from .axl import Axl
from .timeutil import utcify

class StorageInfo:
    current_id = None
    request_start = None
    request_end = None

    def __init__(self, p):
        self.current_id = p.get('current_id')
        self.request_start = p.get('request_start')
        self.request_end = p.get('request_end')

    @staticmethod
    def empty():
        return StorageInfo({})


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
        from dotenv import load_dotenv

        load_dotenv()

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

    def buoy(self, dev: str):
        return next(filter(lambda b: b.matches(dev), self.buoys()))


class Buoy:
    hub: Hub
    dev: str
    name: str
    buoy_type: str

    def __init__(self, hub, dev):
        self.hub = hub

        if isinstance(dev, list):
            self.dev = dev[0]
            self.name = dev[1]
            self.buoy_type = dev[2]
        else:
            self.dev = dev
            self.name = None
            self.buoy_type = 'sfy'

    def __repr__(self):
        return f"Buoy <{self.dev}>"

    def matches(self, key):
        key = key.lower()

        if key in self.dev.lower(): return True
        if self.name is not None:
            if key in self.name.lower(): return True

        return False

    def packages(self):
        return self.hub.__json_request__(self.dev)

    def raw_package(self, pck):
        return self.hub.__request__(f'{self.dev}/{pck}').text

    def json_package(self, pck):
        return self.hub.__json_request__(f'{self.dev}/{pck}')

    def packages_range(self, start=None, end=None):
        """
        Get packages _uploaded_ between start and end datetimes. This is not necessarily the timespan the packages cover.
        """
        if start is None:
            start = 0
        else:
            start = utcify(start)
            start = start.timestamp() * 1000.

        if end is None:
            last = self.last()
            end = last.received * 1000.
        else:
            end = utcify(end)
            end = end.timestamp() * 1000.

        start = math.floor(start)
        end = math.ceil(end)

        path = f"list/{self.dev}/from/{start}/to/{end}"
        pcks = self.hub.__json_request__(path)

        pcks = ((pck.split('-')[0], pck) for pck in pcks)
        pcks = ((datetime.fromtimestamp(float(pck[0]) / 1000.,
                                        tz=timezone.utc), pck[1])
                for pck in pcks)

        return list(pcks)

    def fetch_axl_packages_range(self, start=None, end=None):
        """
        Batch fetch axl packages in range.
        """
        if start is None:
            start = 0
        else:
            start = utcify(start)
            start = start.timestamp() * 1000.

        if end is None:
            last = self.last()
            end = last.received * 1000.
        else:
            end = utcify(end)
            end = end.timestamp() * 1000.

        start = math.floor(start)
        end = math.ceil(end)

        path = f"{self.dev}/from/{start}/to/{end}"
        logger.info(f"Downloading packages between {start} and {end}..")
        return self.hub.__json_request__(path)

    def axl_packages_range(self, start=None, end=None):
        logger.debug(f"fetching axl pacakges between {start} and {end}")

        pcks = self.packages_range(start, end)
        pcks = [pck for pck in pcks if 'axl.qo.json' in pck[1]]
        logger.debug(f"found {len(pcks)} packages, downloading..")

        # download or fetch from cache
        pcks = [self.package(pck[1]) for pck in tqdm(pcks)]
        pcks = [pck for pck in pcks if pck is not None]
        logger.debug(f"dowloaded {len(pcks)} packages.")

        return pcks

    def last(self):
        if self.buoy_type == 'omb':
            return None

        try:
            p = self.hub.__request__(f'{self.dev}/last').text

            return Axl.parse(p)
        except requests.exceptions.HTTPError:
            return None

    def storage_info(self):
        if self.buoy_type == 'omb':
            return StorageInfo.empty()

        try:
            p = self.hub.__json_request__(f'{self.dev}/storage_info')
            return StorageInfo(p)
        except requests.exceptions.HTTPError:
            return StorageInfo.empty()

    def cache_exists(self, pck):
        dev_path = self.hub.cache / self.dev
        pckf: Path = dev_path / pck
        return pckf.exists()

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
                raise

        try:
            return Axl.from_file(pckf)
        except (KeyError, json.decoder.JSONDecodeError) as e:
            # logger.exception(e)
            logger.error(f"failed to parse file: {self.dev}/{pckf}: {e}")
            return None
