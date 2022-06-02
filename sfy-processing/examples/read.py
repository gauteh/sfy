import xarray as xr
import matplotlib.pyplot as plt


ds = xr.open_dataset('test.nc')
print(ds)

z = ds.z
print(z)
