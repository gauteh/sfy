#! /usr/bin/env python

import click
from tabulate import tabulate
from tqdm import tqdm
import matplotlib.pyplot as plt

from sfy.hub import Hub
from sfy import signal


@click.group()
def plotaxl():
    pass


@plotaxl.command(help='Plot package')
@click.argument('dev')
@click.argument('file')
def file(dev, file):
    hub = Hub.from_env()
    buoy = hub.buoy(dev)
    ax = buoy.package(file)

    a = signal.detrend(ax.z)
    _, _, w = signal.velocity(ax)
    _, _, u = signal.displacement(ax)


    plt.figure()
    plt.title(f"Buoy: {buoy.dev}\n{ax.start} / {ax.received_datetime} length: {ax.duration}s f={ax.freq}Hz")
    plt.plot(ax.time[:], a, label = 'acceleration ($m/s^2$)')
    plt.plot(ax.time[:-1], w, label = 'velocity ($m/s$)')
    plt.plot(ax.time[:-2], u, label = 'displacement ($m$)')

    print(ax.time[0])

    plt.grid()
    plt.legend()
    plt.xlabel('Time')
    plt.ylabel('Vertical movement $m$, $m/s$, $m/s^2$')

    plt.show()

if __name__ == '__main__':
    plotaxl()

