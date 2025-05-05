import numpy as np
import xarray as xr
from pathlib import Path
import logging
import pandas as pd

from . import signal
from . import xr as sxr

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

    @property
    def dt(self):
        return 1. / self.frequency

    def extra_attrs(self):
        return {}

    def to_dataset(self, displacement=False, filter_freqs=None, retime=True):
        logger.debug(f'Making xarray Dataset from {self.samples()} samples..')

        ds = xr.Dataset(data_vars={
            'w_z':
            xr.Variable(
                ('time'),
                self.z.astype(np.float32),
                attrs={
                    'unit': 'm/s^2',
                    'long_name': 'sea_water_wave_z_acceleration',
                    'description':
                    'Vertical acceleration (upward, including gravity)',
                    'direction': 'up',
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
                ('package'),
                np.array(self.lons, dtype=np.float64),
                attrs={
                    'units': "degrees_east",
                    'standard_name': "longitude",
                    'long_name': "longitude"
                }),
            'lat':
            xr.Variable(
                ('package'),
                np.array(self.lats, dtype=np.float64),
                attrs={
                    'units': "degrees_north",
                    'standard_name': "latitude",
                    'long_name': "latitude"
                }),
            'package_start':
            xr.Variable(
                ('package'), [
                    np.datetime64(int(s.timestamp() * 1000.),
                                  'ms').astype('datetime64[ns]') if s else None
                    for s in self.start_times
                ],
                attrs={
                    'description':
                    'Timestamp at `offset` sample from the start of each batch (package) of samples.'
                }),
            'offset':
            xr.Variable(
                ('package'),
                self.offsets,
                attrs={
                    'description':
                    'The sample offset in the package where the package_start timestamp is taken.'
                }),
            'added':
            xr.Variable(
                ('package'), [
                    np.datetime64(int(s.timestamp() * 1000.),
                                  'ms').astype('datetime64[ns]') if s else None
                    for s in self.added_times
                ],
                attrs={'description': 'Time package was added to notecard.'}),
            'received':
            xr.Variable(
                ('package'), [
                    np.datetime64(int(s.timestamp() * 1000.),
                                  'ms').astype('datetime64[ns]')
                    for s in self.received_times
                ],
                attrs={'description':
                       'Time package was received by data-hub'}),
            'position_time':
            xr.Variable(
                ('package'), [
                    np.datetime64(int(s), 's').astype('datetime64[ns]')
                    if s else np.nan for s in self.position_times
                ],
                attrs={'description':
                       'Time of position fix for each package'}),
            'storage_id':
            xr.Variable(
                ('package'),
                np.array([
                    id if id is not None else np.nan for id in self.storage_ids
                ]),
                attrs={'description': 'ID of packge on SD-card'}),
        },
                        coords={
                            'time':
                            xr.Variable(
                                ('time'),
                                self.mseconds.astype('datetime64[ms]').astype(
                                    'datetime64[ns]')),
                            'package':
                            xr.Variable(
                                ('package'),
                                np.arange(0, len(self.position_times)),
                                attrs={'description': 'Package number'}),
                        },
                        attrs={
                            'frequency': self.frequency,
                            'frequency:unit': 'Hz',
                            'dt': self.dt,
                            'dt:unit': 's',
                            'package_length': self.package_length,
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
                    'Vertical z-axis displacement (upward, integrated)',
                    'direction': 'up',
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

        # ds = sxr.unique_positions(ds)
        if retime:
            logger.info('Re-timing dataset based on estimated frequency..')
            ds = sxr.retime(ds)
        else:
            fs = np.nanmedian(sxr.estimate_frequency(ds))
            logger.info(f'Not re-timing, estimated frequency to: {fs:.3f} Hz')
            ds = ds.assign_attrs({
                'estimated_frequency': fs,
                'estimated_frequency:unit': 'Hz'
            })

        return ds

    def to_netcdf(self,
                  filename: Path,
                  displacement: bool = False,
                  retime=True):
        """
        Write a CF-compliant NetCDF file to filename.
        """
        compression = {'zlib': True}
        encoding = {}

        ds = self.to_dataset(displacement=displacement, retime=retime)
        for v in ds.variables:
            encoding[v] = compression

        logger.info(f'Writing dataset to {filename}..')
        ds.to_netcdf(filename, format='NETCDF4', encoding=encoding)


class SpecTimeseries:
    def extra_attrs(self):
        return {}

    def to_dataset(self):
        logger.debug(f'Making xarray Dataset from {self.samples()} samples..')

        ds = xr.Dataset(data_vars={
            'A':
            xr.Variable(
                ('time', 'frequency'),
                self.A.astype(np.float32),
                attrs={
                    'unit': 'A^2/Hz',
                    'standard_name': 'acceleration',
                    'direction': 'up',
                }),
            'E':
            xr.Variable(
                ('time', 'frequency'),
                self.A.astype(np.float32),
                attrs={
                    'unit': 'E^2/Hz',
                    'standard_name': 'elevation',
                    'direction': 'up',
                }),
            'added':
            xr.Variable(
                ('package'), [
                    np.datetime64(int(s.timestamp() * 1000.),
                                  'ms').astype('datetime64[ns]') if s else None
                    for s in self.added_times
                ],
                attrs={'description': 'Time package was added to notecard.'}),
            'received':
            xr.Variable(
                ('package'), [
                    np.datetime64(int(s.timestamp() * 1000.),
                                  'ms').astype('datetime64[ns]')
                    for s in self.received_times
                ],
                attrs={'description':
                       'Time package was received by data-hub'}),
        },
                        coords={
                            'time':
                            xr.Variable(('time'), pd.to_datetime(self.time).tz_localize(None).astype(
                                    'datetime64[ns]')),
                            'frequency':
                            xr.Variable(('frequency'),
                                        self.frequency.astype(np.float32)),
                            'package':
                            xr.Variable(
                                ('package'),
                                np.arange(0, len(self)),
                                attrs={'description': 'Package number'}),
                        },
                        attrs={
                            'homepage': 'https://github.com/gauteh/sfy',
                            'buoy_type': 'sfy',
                            'buoy_device': self.device,
                            'buoy_name': self.sn,
                            **self.extra_attrs()
                        })
        print(self.time[0])
        return ds

    def to_netcdf(self, filename: Path):
        """
        Write a CF-compliant NetCDF file to filename.
        """
        compression = {'zlib': True}
        encoding = {}

        ds = self.to_dataset()
        for v in ds.variables:
            encoding[v] = compression

        logger.info(f'Writing dataset to {filename}..')
        ds.to_netcdf(filename, format='NETCDF4', encoding=encoding)


class EgpsTimeseries:

    @property
    def dt(self):
        return 1. / self.frequency

    def extra_attrs(self):
        return {}

    def to_dataset(self):
        logger.debug(f'Making xarray Dataset from {self.samples()} samples..')

        ds = xr.Dataset(data_vars={
            'z':
            xr.Variable(
                ('time'),
                self.z.astype(np.float32),
                attrs={
                    'unit': 'm',
                    'standard_name': 'sea_water_wave_z_elevation',
                    'direction': 'up',
                }),
            'lat':
            xr.Variable(('time'),
                        self.n.astype(np.float32),
                        attrs={
                            'unit': 'degrees_north',
                            'long_name': 'latitude',
                        }),
            'lon':
            xr.Variable(('time'),
                        self.e.astype(np.float32),
                        attrs={
                            'unit': 'degrees_east',
                            'standard_name': 'longitude',
                        }),
            'vz':
            xr.Variable(
                ('time'),
                self.vz.astype(np.float32),
                attrs={
                    'unit': 'mm/s',
                    'standard_name': 'sea_water_wave_z_velocity',
                    'direction': 'up',
                }),
            'vn':
            xr.Variable(('time'),
                        self.vn.astype(np.float32),
                        attrs={
                            'unit': 'mm/s',
                            'standard_name': 'sea_water_wave_north_velocity',
                        }),
            've':
            xr.Variable(('time'),
                        self.ve.astype(np.float32),
                        attrs={
                            'unit': 'mm/s',
                            'standard_name': 'sea_water_wave_east_velocity',
                        }),
            'pck_lon':
            xr.Variable(
                ('package'),
                np.array(self.lons, dtype=np.float64),
                attrs={
                    'units': "degrees_east",
                    'standard_name': "longitude",
                    'long_name': "longitude"
                }),
            'pck_lat':
            xr.Variable(
                ('package'),
                np.array(self.lats, dtype=np.float64),
                attrs={
                    'units': "degrees_north",
                    'standard_name': "latitude",
                    'long_name': "latitude"
                }),
            'package_start':
            xr.Variable(
                ('package'), [
                    np.datetime64(int(s.timestamp() * 1000.),
                                  'ms').astype('datetime64[ns]') if s else None
                    for s in self.start_times
                ],
                attrs={
                    'description':
                    'Timestamp at `offset` sample from the start of each batch (package) of samples.'
                }),
            'package_length':
            xr.Variable(('package'),
                        self.package_length,
                        attrs={'description': 'Length of package'}),
            'added':
            xr.Variable(
                ('package'), [
                    np.datetime64(int(s.timestamp() * 1000.),
                                  'ms').astype('datetime64[ns]') if s else None
                    for s in self.added_times
                ],
                attrs={'description': 'Time package was added to notecard.'}),
            'received':
            xr.Variable(
                ('package'), [
                    np.datetime64(int(s.timestamp() * 1000.),
                                  'ms').astype('datetime64[ns]')
                    for s in self.received_times
                ],
                attrs={'description':
                       'Time package was received by data-hub'}),
            'position_time':
            xr.Variable(
                ('package'), [
                    np.datetime64(int(s), 's').astype('datetime64[ns]')
                    if s else np.nan for s in self.position_times
                ],
                attrs={'description':
                       'Time of position fix for each package'}),
        },
                        coords={
                            'time':
                            xr.Variable(
                                ('time'),
                                self.mseconds.astype('datetime64[ms]').astype(
                                    'datetime64[ns]')),
                            'package':
                            xr.Variable(
                                ('package'),
                                np.arange(0, len(self.position_times)),
                                attrs={'description': 'Package number'}),
                        },
                        attrs={
                            'frequency': self.frequency,
                            'frequency:unit': 'Hz',
                            'dt': self.dt,
                            'dt:unit': 's',
                            'package_length': self.package_length,
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
        compression = {'zlib': True}
        encoding = {}

        ds = self.to_dataset()
        for v in ds.variables:
            encoding[v] = compression

        logger.info(f'Writing dataset to {filename}..')
        ds.to_netcdf(filename, format='NETCDF4', encoding=encoding)
