#! /usr/bin/env python

import click
import matplotlib.pyplot as plt
from datetime import timedelta

from sfy.hub import Hub
from sfy import signal


@click.group()
def axl():
    pass


@axl.command(help='Plot package')
@click.argument('dev')
@click.argument('file')
def file(dev, file):
    hub = Hub.from_env()
    buoy = hub.buoy(dev)
    ax = buoy.package(file)

    a = signal.detrend(ax.z)
    _, _, w = signal.velocity(ax)
    _, _, u = signal.displacement(ax)
    u = signal.detrend(u)

    plt.figure()
    plt.title(
        f"Buoy: {buoy.dev}\n{ax.start} / {ax.received_datetime} length: {ax.duration}s f={ax.freq}Hz"
    )
    plt.plot(ax.time[:], a, label='acceleration ($m/s^2$)')
    plt.plot(ax.time[:-1], w, label='velocity ($m/s$)')
    plt.plot(ax.time[:-2], u, label='displacement ($m$)')

    print(ax.time[0])

    plt.grid()
    plt.legend()
    plt.xlabel('Time')
    plt.ylabel('Vertical movement $m$, $m/s$, $m/s^2$')

    plt.show()

@axl.command(help='Monitor buoy')
@click.argument('dev')
@click.option('--sleep',
              help='Time to sleep between update',
              default=5.0,
              type=float)
@click.option('--window',
              help='Time window to show',
              default=None,
              type=float)
def monitor(dev, sleep, window):
    hub = Hub.from_env()
    buoy = hub.buoy(dev)

    fig = plt.figure()
    ax = fig.add_subplot(111)
    plt.grid()
    plt.legend()
    plt.xlabel('Time')
    plt.ylabel('Vertical movement $m$, $m/s$, $m/s^2$')

    la = None
    lv = None
    lu = None

    axl = None

    while True:
        naxl = buoy.last()
        print(naxl.time[0])

        if axl is None or axl.start != naxl.start:
            print("new data package")
            axl = naxl

            plt.title(
                f"Buoy: {buoy.dev}\n{axl.start} / {axl.received_datetime} length: {axl.duration}s f={axl.freq}Hz"
            )

            a = signal.detrend(axl.z)
            _, _, w = signal.velocity(axl)
            _, _, u = signal.displacement(axl)

            if la is None:
                la, = ax.plot(axl.time[:], a, 'k--', alpha=.5, label='acceleration ($m/s^2$)')
                lv, = ax.plot(axl.time[:-1], w, 'g--', alpha=.5, label='velocity ($m/s$)')
                lu, = ax.plot(axl.time[:-2], u, 'b', label='displacement ($m$)')
            else:
                la.set_data(axl.time[:], a)
                lv.set_data(axl.time[:-1], w)
                lu.set_data(axl.time[:-2], u)

        plt.legend()

        if window is not None:
            plt.xlim([axl.end - timedelta(seconds=window), axl.end])


        plt.pause(sleep)

