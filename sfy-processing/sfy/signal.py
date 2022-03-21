import scipy as sc, scipy.signal, scipy.integrate

from .axl import Axl

def detrend(s):
    return sc.signal.detrend(s)

def integrate(s, dt):
    return sc.integrate.cumtrapz(s, dx = dt)

def velocity(a: Axl):
    """
    Calculate velocity from axelerometer package. Resulting vectors will be one length shorter than the original.
    """
    x, y, z = a.x, a.y, a.z
    x = detrend(x)
    y = detrend(y)
    z = detrend(z)

    x = integrate(x, a.dt)
    y = integrate(y, a.dt)
    z = integrate(z, a.dt)

    return x, y, z

def displacement(a: Axl):
    """
    Calculate diplacement from axelerometer package. Resulting vectors will be two length shorter than the original.
    """
    x, y, z = velocity(a)

    x = integrate(x, a.dt)
    y = integrate(y, a.dt)
    z = integrate(z, a.dt)

    return x, y, z


