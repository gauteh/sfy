import os
from pathlib import Path
from urllib.parse import urljoin
import requests
from datetime import datetime, timezone
import logging
from tqdm import tqdm
import math
import tempfile
import base64

logger = logging.getLogger(__name__)

from .axl import Axl
from .spec import Spec
from .egps import Egps
from .event import Event
from .timeutil import utcify


class Hub:
    endpoint: str
    key: str
    cache: Path

    tmpdir = None

    def __init__(self, endpoint, key, cache=None):
        """
        Set up a Hub client.

            endpoint: URL to sfy hub including buoys, e.g.:

                https://wavebug.met.no/buoys/

            key: Read token.

            cache: A directory used to cache the data files.
        """
        self.endpoint = endpoint

        if self.endpoint[-1] != '/':
            self.endpoint += '/'

        self.key = key

        if cache is not None:
            self.cache = Path(cache)
        else:
            logger.error(
                "No cache dir specified, will use temporary directory. This will cause data to be re-downloaded every time."
            )
            self.tmpdir = tempfile.TemporaryDirectory()
            self.cache = Path(self.tmpdir.name)

        if not self.cache.exists():
            os.makedirs(self.cache, exist_ok=True)

    @staticmethod
    def from_env():
        from urllib.parse import urljoin

        API = os.getenv('SFY_SERVER')
        KEY = os.getenv('SFY_READ_TOKEN')
        CACHE = os.getenv('SFY_DATA_CACHE')

        if API is None or KEY is None:
            raise Exception("Missing API or KEY")

        API = urljoin(API, 'buoys')
        return Hub(urljoin(API, 'buoys'), KEY, CACHE)

    def __request__(self, path):
        url = urljoin(self.endpoint, path)

        logger.debug(f'Requesting: {url}')

        r = requests.get(url, headers={'SFY_AUTH_TOKEN': self.key})
        r.raise_for_status()

        return r

    def __json_request__(self, path):
        return self.__request__(path).json()

    def buoys(self):
        """
        Get list of buoys.
        """
        return [Buoy.from_hub(self, d) for d in self.__json_request__('./')]

    def buoy(self, dev: str):
        """
        Return last buoy matching `dev` in `sn` or `dev`, sorted by last contact.
        """
        buoys = self.buoys()

        last = [b.last() if 'lost+found' not in b.dev else None for b in buoys]
        last = [l.received_datetime if l else None for l in last]

        buoys = [[b, l] for b, l in zip(buoys, last)]
        buoys.sort(key=lambda b: -b[1].timestamp() if b[1] else 0)

        b = next(filter(lambda b: b[0].matches(dev), buoys))[0]
        return b

    def login(self):
        """
        Login to notehub to get token.
        """

        logger.debug('Logging into Notehub..')
        user = os.getenv('SFY_NH_USER')
        pw = os.getenv('SFY_NH_PW')
        assert user is not None and pw is not None, "SFY_NH_USER and/or SFY_NH_PW environment variables with credentials to Notehub.io not set."

        r = requests.post('https://api.notefile.net/auth/login',
                          json={
                              'username': user,
                              'password': pw
                          })
        r.raise_for_status()

        return r.json()['session_token']


class Buoy:
    hub: Hub
    dev: str
    name: str
    buoy_type: str
    __last__: str

    @staticmethod
    def from_hub(hub, dev):
        """
        Construct a correct Buoy type from from a hub entry.
        """

        from .omb import OmbBuoy

        buoy_type = dev[2]

        if buoy_type == 'sfy':
            return SfyBuoy(hub, *dev)
        elif buoy_type == 'omb':
            return OmbBuoy(hub, *dev)
        else:
            return None

    def __repr__(self):
        return f"Buoy {self.name} <{self.dev}> ({self.buoy_type})"

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

        pcks = ((pck[0].split('-')[0], *pck) for pck in pcks)
        pcks = ((datetime.fromtimestamp(float(pck[0]) / 1000.,
                                        tz=timezone.utc), pck[1])
                for pck in pcks)

        return list(pcks)

    def fetch_packages_range(self, start=None, end=None):
        """
        Batch fetch packages in range.

        Returns a list of lists with:

            [ received(ms), event, payload(text) ]
        """
        # find first un-cached package
        logger.debug('Fetching list of packages..')
        list_pcks = self.packages_range(start, end)
        try:
            fu_i, first_uncached = next(
                filter(lambda p: not self.cache_path(p[1][1]).exists(),
                       enumerate(list_pcks)))
        except StopIteration:
            logger.debug('All packages already cached.')
            fu_i, first_uncached = len(list_pcks), None

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

        if first_uncached is not None:
            start = math.floor(first_uncached[0].timestamp() * 1000.)
            logger.debug(f'Packages before {start} already cached.')

            path = f"{self.dev}/from/{start}/to/{end}"
            logger.info(f"Downloading packages between {start} and {end}..")

            pcks = self.hub.__json_request__(path)
            pcks = [[
                p['received'], f"{p['received']}-{p['event']}",
                base64.b64decode(p['data'])
            ] for p in pcks]

            logger.info(f'Downloaded {len(pcks)} packages..')

            # write to cache
            dev_path = self.hub.cache / self.dev
            os.makedirs(dev_path, exist_ok=True)

            for pck in pcks:
                pckf: Path = dev_path / pck[1]
                if not pckf.exists():
                    with open(pckf, 'wb') as fd:
                        fd.write(pck[2])
        else:
            pcks = []

        # prepend already cached packages
        logger.debug(f'Loading {fu_i} cached packages from disk..')
        np = [[p[0].timestamp() * 1000., p[1],
               self.fetch_package(p[1])] for p in list_pcks[:fu_i]]
        logger.debug(f'Prepending cached packages: {len(np)}')
        np.extend(pcks)
        pcks = np

        return pcks

    def last(self):
        if self.buoy_type == 'omb':
            return self.__last__
        else:
            return self.__last__

    def cache_path(self, event):
        dev_path = self.hub.cache / self.dev
        os.makedirs(dev_path, exist_ok=True)
        pckf: Path = dev_path / event
        return pckf

    def fetch_package(self, pck):
        """
        Fetch a package if it does not exist in the cache.
        """
        dev_path = self.hub.cache / self.dev
        os.makedirs(dev_path, exist_ok=True)

        pckf: Path = dev_path / pck
        if not pckf.exists():
            try:
                with open(pckf, 'w') as fd:
                    pck = self.hub.__request__(f'{self.dev}/{pck}').text
                    fd.write(pck)
            except:
                os.remove(pckf)
                raise
        else:
            pck = open(pckf).read()

        return pck

    def package(self, pck):
        """
        Fetch and parse an Axl package.
        """
        pck = self.fetch_package(pck)
        return Axl.try_parse(pck)


class SfyBuoy(Buoy):

    def __init__(self, hub, dev, name, buoy_type, last):
        self.buoy_type = 'sfy'
        assert buoy_type == self.buoy_type

        self.hub = hub
        self.dev = dev
        self.name = name
        if len(last) > 0:
            self.__last__ = Event.parse(base64.b64decode(last))
        else:
            self.__last__ = None

    def axl_packages_range(self, start=None, end=None):
        logger.debug(f"fetching axl packages between {start} and {end}")

        pcks = self.fetch_packages_range(start, end)
        pcks = [pck for pck in pcks if 'axl.qo.json' in pck[1]]
        logger.debug(f"Found {len(pcks)} axl packages")

        pcks = [Axl.try_parse(pck[2]) for pck in tqdm(pcks)]
        pcks = [pck for pck in pcks if pck is not None]
        logger.debug(f"Loaded {len(pcks)} packages.")

        return pcks

    def spec_packages_range(self, start=None, end=None):
        logger.debug(f"fetching spec packages between {start} and {end}")

        pcks = self.fetch_packages_range(start, end)
        pcks = [pck for pck in pcks if 'spec.qo.json' in pck[1]]
        logger.debug(f"Found {len(pcks)} spec packages")

        pcks = [Spec.try_parse(pck[2]) for pck in tqdm(pcks)]
        pcks = [pck for pck in pcks if pck is not None]
        logger.debug(f"Loaded {len(pcks)} packages.")

        return pcks

    def egps_packages_range(self, start=None, end=None):
        logger.debug(f"fetching egps packages between {start} and {end}")

        pcks = self.fetch_packages_range(start, end)
        pcks = [pck for pck in pcks if 'egps.qo.json' in pck[1]]
        logger.debug(f"Found {len(pcks)} egps packages")

        pcks = [Egps.try_parse(pck[2]) for pck in tqdm(pcks)]
        pcks = [pck for pck in pcks if pck is not None]
        logger.debug(f"Loaded {len(pcks)} packages.")

        return pcks

    def position_packages_range(self, start=None, end=None, only_axl=False):
        """
        Get all packages that contain position information.

            only_axl: Only include positions from `axl` packages. Useful for determining where data has been collected.
        """
        logger.debug(f"fetching position packages between {start} and {end}")
        pcks = self.fetch_packages_range(start, end)

        pcks = (pck for pck in pcks
                if 'axl.qo.json' in pck[1] or '_track.qo.json' in pck[1])
        if only_axl:
            pcks = (pck for pck in pcks if 'axl.qo.json' in pck[1])

        pcks = list(pcks)
        logger.debug(f"Found {len(pcks)} position packages")

        pcks = [Event.try_parse(pck[2]) for pck in tqdm(pcks)]
        pcks = [pck for pck in pcks if pck is not None]
        logger.debug(f"Loaded {len(pcks)} packages.")

        return pcks

