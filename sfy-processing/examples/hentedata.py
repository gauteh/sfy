import datetime
from sfy.hub import Hub
from sfy import signal

start = datetime.datetime(2022, 4, 25, hour=12)
end = datetime.datetime(2022, 4, 25, hour=12, minute=10)

dev = '78'

hub = Hub.from_env()
buoy = hub.buoy(dev)

packages = buoy.packages_range(start, end)
print(packages)

for i in range(0,len(packages)):
    pck0 = packages[i][1]
    print(pck0)

    ax = buoy.package(pck0)
    print(ax.start)
    print(ax.z)
