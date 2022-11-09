import numpy as np
import xarray as xr
from pathlib import Path
from . import signal


class AxlTimeseries:
    def displacement(self):
        """
        Integrate to displacement using default parameters.
        """
        filter_freqs = signal.DEFAULT_BANDPASS_FREQS.copy()
        if self.frequency < 50:
            filter_freqs = [filter_freqs[0], 10.]

        u_z = signal.integrate(self.z, self.dt, order=2, freqs=filter_freqs, method='dft')
        u_x = signal.integrate(self.x, self.dt, order=2, freqs=filter_freqs, method='dft')
        u_y = signal.integrate(self.y, self.dt, order=2, freqs=filter_freqs, method='dft')

        return (u_z, u_x, u_y, filter_freqs)

    @property
    def dt(self):
        return 1. / self.frequency

    def extra_attrs(self):
        return {}

    def to_dataset(self):
        u_z, u_x, u_y, filter_freqs = self.displacement()

        ds = xr.Dataset(
            data_vars={
                'w_z':
                xr.Variable(
                    ('time'),
                    self.z.astype(np.float32),
                    attrs={
                        'unit': 'm/s^2',
                        'long_name': 'sea_water_wave_z_acceleration',
                        'description':
                        'Vertical acceleration (including gravity)'
                    }),
                'w_x':
                xr.Variable(
                    ('time'),
                    self.x.astype(np.float32),
                    attrs={
                        'unit': 'm/s^2',
                        'long_name': 'sea_water_wave_x_acceleration',
                        'description': 'Horizontal x-axis acceleration'
                    }),
                'w_y':
                xr.Variable(
                    ('time'),
                    self.y.astype(np.float32),
                    attrs={
                        'unit': 'm/s^2',
                        'long_name': 'sea_water_wave_y_acceleration',
                        'description': 'Horizontal y-axis acceleration'
                    }),
                'u_z':
                xr.Variable(
                    ('time'),
                    u_z.astype(np.float32),
                    attrs={
                        'unit': 'm',
                        'long_name': 'sea_water_wave_z_displacement',
                        'description': 'Horizontal z-axis displacement (integrated)',
                        'detrended': 'yes',
                        'filter': 'butterworth (10th order), two-ways',
                        'filter_freqs': filter_freqs,
                        'filter_freqs:unit': 'Hz',
                    }),
                'u_x':
                xr.Variable(
                    ('time'),
                    u_x.astype(np.float32),
                    attrs={
                        'unit': 'm',
                        'long_name': 'sea_water_wave_x_displacement',
                        'description': 'Horizontal x-axis displacement (integrated)',
                        'detrended': 'yes',
                        'filter': 'butterworth (10th order), two-ways',
                        'filter_freqs': filter_freqs,
                        'filter_freqs:unit': 'Hz',
                    }),
                'u_y':
                xr.Variable(
                    ('time'),
                    u_y.astype(np.float32),
                    attrs={
                        'unit': 'm',
                        'long_name': 'sea_water_wave_y_displacement',
                        'description': 'Horizontal y-axis displacement (integrated)',
                        'detrended': 'yes',
                        'filter': 'butterworth (10th order), two-ways',
                        'filter_freqs': filter_freqs,
                        'filter_freqs:unit': 'Hz',
                    }),
                'lon':
                xr.Variable(
                    ('position_time'),
                    np.array(self.lons, dtype=np.float64),
                    attrs={
                        'units': "degrees_east",
                        'standard_name': "longitude",
                        'long_name': "longitude"
                    }),
                'lat':
                xr.Variable(
                    ('position_time'),
                    np.array(self.lats, dtype=np.float64),
                    attrs={
                        'units': "degrees_north",
                        'standard_name': "latitude",
                        'long_name': "latitude"
                    }),
                'package_start':
                xr.Variable(('received'), [
                    np.datetime64(int(s.timestamp() * 1000.), 'ms') if s else None
                    for s in self.start_times
                ],
                            attrs={
                                'description':
                                'Timestamp at start of each batch (package) of samples.'
                            }),
                'added':
                xr.Variable(('received'), [
                    np.datetime64(int(s.timestamp() * 1000.), 'ms') if s else None
                    for s in self.added_times
                ],
                            attrs={
                                'description':
                                'Time package was added to notecard.'
                            }),
                'storage_id':
                xr.Variable(('received'),
                            np.array([
                                id if id is not None else np.nan
                                for id in self.storage_ids
                            ]),
                            attrs={'description': 'ID of packge on SD-card'}),
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
                    [np.datetime64(int(s), 's') if s else np.nan for s in self.position_times],
                    attrs={
                        'description': 'Time of position fix for each package'
                    }),
                'received':
                xr.Variable(('received'), [
                    np.datetime64(int(s.timestamp() * 1000.), 'ms')
                    for s in self.received_times
                ],
                            attrs={
                                'description':
                                'Time package was received by data-hub'
                            }),
            },
            attrs={
                'frequency': self.frequency,
                'frequency:unit': 'Hz',
                'dt': self.dt,
                'dt:unit': 's',
                'homepage': 'https://github.com/gauteh/sfy',
                'buoy_type': 'sfy',
                'buoy_device': self.device,
                'buoy_name': self.sn,
                **self.extra_attrs()
            })

        # Adjust for on-board FIR filter
        ds = signal.adjust_fir_filter(ds)

        return ds

    def to_netcdf(self, filename: Path):
        """
        Write a CF-compliant NetCDF file to filename.
        """
        self.to_dataset().to_netcdf(filename, format='NETCDF4')
