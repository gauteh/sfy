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

    def __init__(self, hub, dev):
        self.hub = hub

        if isinstance(dev, list):
            self.dev = dev[0]
            self.name = dev[1]
        else:
            self.dev = dev
            self.name = None

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

        data_pcks = []

        print(len(pcks))
        i = 0
        while i < len(pcks):
            print("o", i)
            p = pcks[i]
            if self.cache_exists(p[1]):
                data_pcks.append(self.package(p[1]))
                i += 1
            else:
                # find next cached package or end
                j = i + 1
                while j < len(pcks):
                    print("i", i, j)
                    pj = pcks[j]
                    if self.cache_exists(pj[1]):
                        j -= 1
                        break
                    else:
                        # continue search
                        j += 1

                if j == len(pcks): j -= 1

                # fetch range from i to j
                npcks = self.fetch_axl_packages_range(
                    pcks[i][0], pcks[j][0])
                data_pcks.extend(npcks)

                i = j+1

        # print(bytearray(data_pcks[0]['data']))

        # data_pcks = [Axl.parse(bytearray(d['data'])) for d in data_pcks]

        for pd, dd in zip(pcks, data_pcks):
            print(pd[0].timestamp(), dd['received'])

        print(len(data_pcks))
        u = set((p['received'] for p in data_pcks))
        print(len(u))
        assert len(u) == len(data_pcks)
        assert len(pcks) == len(data_pcks)

        pcks = [[p[1], d] for p,d in zip(pcks, data_pcks)]
        pcks = [pck for pck in pcks if pck is not None]
        logger.debug(f"dowloaded {len(pcks)} packages.")

        return pcks

    def last(self):
        p = self.hub.__request__(f'{self.dev}/last').text

        return Axl.parse(p)

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
        except json.decoder.JSONDecodeError as e:
            # logger.exception(e)
            logger.error(f"failed to parse file: {self.dev}/{pckf}: {e}")
            return None
