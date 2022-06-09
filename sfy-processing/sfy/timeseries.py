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

    def extra_attrs(self):
        return {}

    def to_dataset(self):
        ds = xr.Dataset(
            data_vars={
                'z':
                xr.Variable(('time'), self.z.astype(np.float32)),
                'x':
                xr.Variable(('time'), self.x.astype(np.float32)),
                'y':
                xr.Variable(('time'), self.y.astype(np.float32)),
                'lon':
                xr.Variable(('position_time'), np.array(self.lons, dtype=np.float64)),
                'lat':
                xr.Variable(('position_time'), np.array(self.lats, dtype=np.float64)),
                'added':
                xr.Variable(('received'), [
                    np.datetime64(int(s.timestamp() * 1000.), 'ms')
                    for s in self.added_times
                ]),
                'storage_id':
                xr.Variable(('received'),
                            np.array([
                                id if id is not None else np.nan
                                for id in self.storage_ids
                            ])),
            },
            coords={
                'time':
                xr.Variable(('time'), [
                    np.datetime64(int(s.timestamp() * 1000.), 'ms')
                    for s in self.time
                ]),
                'position_time':
                xr.Variable(
                    'position_time',
                    [np.datetime64(int(s), 'ms') for s in self.position_times],
                    attrs={
                        'description': 'Time of position fix for each package'
                    }),
                'received':
                xr.Variable(('received'), [
                    np.datetime64(int(s.timestamp() * 1000.), 'ms')
                    for s in self.received_times
                ]),
            },
            attrs={
                'frequency': self.frequency,
                'frequency:unit': 'Hz',
                'homepage': 'https://github.com/gauteh/sfy',
                'buoy_type': 'sfy',
                'buoy_device': self.device,
                'buoy_name': self.sn,
                **self.extra_attrs()
            })

        return ds

    def to_netcdf(self, filename: Path):
        """
        Write a CF-compliant NetCDF file to filename.
        """
        self.to_dataset().to_netcdf(filename, format='NETCDF4')
