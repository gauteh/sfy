import base64
import json

from .hub import Buoy

class OmbBuoy(Buoy):

    def __init__(self, hub, dev, name, buoy_type, last):
        self.buoy_type = 'omb'
        assert buoy_type == self.buoy_type

        self.hub = hub
        self.dev = dev
        self.name = name
        if len(last) > 0:
            self.__last__ = json.loads(base64.b64decode(last))
        else:
            self.__last__ = None

