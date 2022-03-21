from urllib.parse import urljoin
import requests

from .axl import Axl


class Hub:
    endpoint: str
    key: str

    def __init__(self, endpoint, key):
        """
        Set up a Hub client.

            endpoint: URL to sfy hub.

            key: Read token.
        """
        self.endpoint = endpoint

        if self.endpoint[-1] != '/':
            self.endpoint += '/'

        self.key = key

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

    def package(self, pck):
        return Axl.parse(self.hub.__request__(f'{self.dev}/{pck}').text)
