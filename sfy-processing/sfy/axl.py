from dataclasses import dataclass
import json
import numpy as np
import base64
import sys
import logging
logger = logging.getLogger(__name__)

@dataclass
class Axl:
    received: float
    routed: float

    event: str
    session: str
    product: str
    req: str
    when: int
    file: str
    updates: int

    device: str
    sn: str

    tower_when: int
    tower_lon: float
    tower_lat: float
    tower_country: str
    tower_location: str
    tower_timezone: str
    tower_id: str

    project: dict

    ## Payload and body
    length: int
    offset: int
    timestamp: int
    lon: float
    lat: float

    x: np.ndarray
    y: np.ndarray
    z: np.ndarray

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
        data['lon'] = data['body'].get('lon')
        data['lat'] = data['body'].get('lat')
        del data['body']

        # decode x, y, z
        payload = payload[:data['length']]
        payload = base64.b64decode(payload)
        payload = np.frombuffer(payload, dtype=np.float16)

        if sys.byteorder == 'big':
            logger.warning('host is big-endian, swapping bytes: this is not well-tested.')
            payload.byteswap(inplace = True)

        x = payload[0::3]
        y = payload[1::3]
        z = payload[2::3]

        return Axl(**data, x=x, y=y, z=z)

