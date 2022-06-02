import numpy as np
import xarray as xr
from pathlib import Path

class AxlTimeseries:
    def z_spectrum(self):
        """
        Calculate spectrum
        """
        pass

    def z_spectrogram(self):
        """
        Calculate spectrogram
        """
        pass

    def to_netcdf(self, filename: Path):
        """
        Write a CF-compliant NetCDF file to filename.
        """
        ds = xr.Dataset({'z': (('time'), self.z.astype(np.float32))},
                        coords={'time': self.mseconds})
        ds.to_netcdf(filename)

