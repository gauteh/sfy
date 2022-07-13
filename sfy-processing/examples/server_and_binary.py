import datetime
from sfy.hub import Hub
from sfy.axl import AxlCollection

start = datetime.datetime(2022, 4, 25, hour=12)
end = datetime.datetime(2022, 4, 25, hour=12, minute=10)

dev = '278'

hub = Hub.from_env()
buoy = hub.buoy(dev)

packages = buoy.axl_packages_range(start, end)

server_pcks = AxlCollection(packages)
sd_pcks = AxlCollection.from_storage_file(buoy.name, buoy.dev, '../sfy-buoy/tests/data/74.1')

pcks = server_pcks + sd_pcks

assert len(pcks) == (len(server_pcks) + len(sd_pcks))

print(pcks.to_dataset())
