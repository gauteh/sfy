import numpy as np
import xarray as xr
from pathlib import Path
import logging
from concurrent.futures import ThreadPoolExecutor

from . import signal

logger = logging.getLogger(__name__)


class AxlTimeseries:
    z: np.ndarray
    x: np.ndarray
    y: np.ndarray
    frequency: float

    def default_bandpass_freqs(self):
        if self.frequency < 50:
            return signal.DEFAULT_BANDPASS_FREQS_20Hz
        else:
            return signal.DEFAULT_BANDPASS_FREQS_52Hz

    def displacement(self, filter_freqs=None):
        """
        Integrate to displacement using default parameters.
        """
        if filter_freqs is None:
            filter_freqs = self.default_bandpass_freqs()

        logger.info(
            f'Integrating displacment, filter frequencies: {filter_freqs}.')

        u_z = signal.integrate(self.z,
                               self.dt,
                               order=2,
                               freqs=filter_freqs,
                               method='dft')
        u_x = signal.integrate(self.x,
                               self.dt,
                               order=2,
                               freqs=filter_freqs,
                               method='dft')
        u_y = signal.integrate(self.y,
                               self.dt,
                               order=2,
                               freqs=filter_freqs,
                               method='dft')

        return (u_z, u_x, u_y, filter_freqs)

    def hm0(self, raw=False, window=(20 * 60)):
        """
        Return DataArray with Hm0.
        """

        z = self.z

        # split into windows
        N = int(window * self.frequency)
        N = min(N, len(z))
        i = np.arange(N, len(z), N).astype(np.int32)
        z = np.split(z, i)

        # Calculate hm0 for each window
        logger.debug('Calculating Hm0 for all 20 minute segments..')

        def hm0(zz):
            f, P = signal.welch(self.frequency, zz)
            if not raw:
                _, _, P = signal.imu_cutoff_rabault2022(f, P)
            return signal.hm0(f, P)

        with ThreadPoolExecutor() as x:
            hm0 = list(x.map(hm0, z))

        i = np.append(i, len(z))

        time = self.mseconds[i - N].astype('datetime64[ms]')

        logger.debug('Building dataarray..')
        return xr.DataArray(
            hm0,
            coords=[('time', time)],
            attrs={
                'unit':
                'm',
                'long_name':
                'sea_surface_wave_significant_height',
                'description':
                'Significant wave height calculated in the frequency domain from the first moment.'
            })

    @property
    def dt(self):
        return 1. / self.frequency

    def extra_attrs(self):
        return {}

    def to_dataset(self, displacement=False, filter_freqs=None):
        logger.debug(f'Making xarray Dataset from {self.samples()} samples..')

        ds = xr.Dataset(data_vars={
            'w_z':
            xr.Variable(
                ('time'),
                self.z.astype(np.float32),
                attrs={
                    'unit': 'm/s^2',
                    'long_name': 'sea_water_wave_z_acceleration',
                    'description': 'Vertical acceleration (including gravity)'
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
            xr.Variable(
                ('received'), [
                    np.datetime64(int(s.timestamp() *
                                      1000.), 'ms') if s else None
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
            xr.Variable(
                ('received'),
                np.array([
                    id if id is not None else np.nan for id in self.storage_ids
                ]),
                attrs={'description': 'ID of packge on SD-card'}),
        },
                        coords={
                            'time':
                            xr.Variable(
                                ('time'),
                                self.mseconds.astype('datetime64[ms]')),
                            'position_time':
                            xr.Variable(
                                'position_time', [
                                    np.datetime64(int(s), 's') if s else np.nan
                                    for s in self.position_times
                                ],
                                attrs={
                                    'description':
                                    'Time of position fix for each package'
                                }),
                            'received':
                            xr.Variable(
                                ('received'), [
                                    np.datetime64(int(s.timestamp() * 1000.),
                                                  'ms')
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

        if self.has_raw:
            ds['a_x'] = xr.Variable(
                ('time'),
                self.ax.astype(np.float32),
                attrs={
                    'unit': 'm/s^2',
                    'long_name': 'sea_water_wave_x_acceleration',
                    'description': 'Raw acceleration measured by IMU'
                })

            ds['a_y'] = xr.Variable(
                ('time'),
                self.ay.astype(np.float32),
                attrs={
                    'unit': 'm/s^2',
                    'long_name': 'sea_water_wave_y_acceleration',
                    'description': 'Raw acceleration measured by IMU'
                })
            ds['a_z'] = xr.Variable(
                ('time'),
                self.az.astype(np.float32),
                attrs={
                    'unit': 'm/s^2',
                    'long_name': 'sea_water_wave_z_acceleration',
                    'description': 'Raw acceleration measured by IMU'
                })

            ds['g_x'] = xr.Variable(
                ('time'),
                self.gx.astype(np.float32),
                attrs={
                    'unit': 'rad',
                    'long_name': 'gyro',
                    'description': 'Raw gyro measured by IMU'
                })

            ds['g_y'] = xr.Variable(
                ('time'),
                self.gy.astype(np.float32),
                attrs={
                    'unit': 'rad',
                    'long_name': 'gyro',
                    'description': 'Raw gyro measured by IMU'
                })
            ds['g_z'] = xr.Variable(
                ('time'),
                self.gz.astype(np.float32),
                attrs={
                    'unit': 'rad',
                    'long_name': 'gyro',
                    'description': 'Raw gyro measured by IMU'
                })



        if displacement:
            u_z, u_x, u_y, filter_freqs = self.displacement(filter_freqs)
            ds['u_z'] = xr.Variable(
                ('time'),
                u_z.astype(np.float32),
                attrs={
                    'unit': 'm',
                    'long_name': 'sea_water_wave_z_displacement',
                    'description':
                    'Horizontal z-axis displacement (integrated)',
                    'detrended': 'yes',
                    'filter': 'butterworth (10th order), two-ways',
                    'filter_freqs': filter_freqs,
                    'filter_freqs:unit': 'Hz',
                })

            ds['u_x'] = xr.Variable(
                ('time'),
                u_x.astype(np.float32),
                attrs={
                    'unit': 'm',
                    'long_name': 'sea_water_wave_x_displacement',
                    'description':
                    'Horizontal x-axis displacement (integrated)',
                    'detrended': 'yes',
                    'filter': 'butterworth (10th order), two-ways',
                    'filter_freqs': filter_freqs,
                    'filter_freqs:unit': 'Hz',
                })
            ds['u_y'] = xr.Variable(
                ('time'),
                u_y.astype(np.float32),
                attrs={
                    'unit': 'm',
                    'long_name': 'sea_water_wave_y_displacement',
                    'description':
                    'Horizontal y-axis displacement (integrated)',
                    'detrended': 'yes',
                    'filter': 'butterworth (10th order), two-ways',
                    'filter_freqs': filter_freqs,
                    'filter_freqs:unit': 'Hz',
                })

        # Adjust for on-board FIR filter
        logger.debug('Adjusting for FIR filter delay')
        ds = signal.adjust_fir_filter(ds)

        return ds

    def to_netcdf(self, filename: Path, displacement: bool = False):
        """
        Write a CF-compliant NetCDF file to filename.
        """
        compression = {'zlib': True}
        encoding = {}

        ds = self.to_dataset(displacement=displacement)
        for v in ds.variables:
            encoding[v] = compression

        ds.to_netcdf(filename, format='NETCDF4', encoding=encoding)
