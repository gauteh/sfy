from concurrent.futures import ThreadPoolExecutor
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

        raw = False
        f0 = 0.05

        freq = self.frequency
        E = self.E

        def stat(E):
            fc = 0.0
            i0, fc, pE = signal.imu_cutoff_rabault2022(freq, E, f0)
            return *signal.spec_stats(freq, pE), fc, pE

        with ThreadPoolExecutor() as ex:
            m_1, m0, m1, m2, m4, hm0, Tm01, Tm02, Tm_10, Tp, fc, pE = zip(
                *ex.map(stat, E))

        pE = np.vstack(pE)
        # spec = np.vectorize(signal.spec_stats, excluded={0, 'f'})
        # m_1, m0, m1, m2, m4, hm0, Tm01, Tm02, Tm_10, Tp = spec(self.f, E)

        ds = xr.Dataset(data_vars={
            'A':
            xr.Variable(
                ('time', 'frequency'),
                self.A.astype(np.float32),
                attrs={
                    'unit':
                    'm^2/s^4/Hz',
                    'description':
                    'Sea surface acceleration spectrum calculated using Welch method.',
                }),
            'E':
            xr.Variable(
                ('time', 'frequency'),
                E.astype(np.float32),
                attrs={
                    'unit':
                    'm^2/Hz',
                    'standard_name':
                    'sea_surface_wave_variance_spectral_density',
                    'description':
                    'Sea surface elevation spectrum (variance density spectrum) calculated using Welch method.',
                }),

            'hm0':
            xr.DataArray(
                np.array(hm0),
                dims=[
                    'time',
                ],
                attrs={
                    'unit':
                    'm',
                    'standard_name':
                    'sea_surface_wave_significant_height',
                    'description':
                    'Significant wave height calculated in the frequency domain from the zeroth moment (4 * sqrt(m0)).'
                }),
            'fc':
            xr.DataArray(
                np.array(fc),
                dims=[
                    'time',
                ],
                attrs={
                    'unit':
                    'Hz',
                    'description':
                    'High-pass cut-off frequency used in spectral statistics.'
                }),
            'Tm01':
            xr.DataArray(
                np.array(Tm01),
                dims=['time'],
                attrs={
                    'unit': 's',
                    'standard_name':
                    'sea_surface_wave_mean_period_from_variance_spectral_density_first_frequency_moment',
                    'description': 'First wave period (m0/m1)'
                }),
            'Tm02':
            xr.DataArray(
                np.array(Tm02),
                dims=['time'],
                attrs={
                    'unit': 's',
                    'standard_name':
                    'sea_surface_wave_mean_period_from_variance_spectral_density_second_frequency_moment',
                    'description': 'Second wave period (sqrt(m0/m2))'
                }),
            'Tm_10':
            xr.DataArray(
                np.array(Tm_10),
                dims=['time'],
                attrs={
                    'unit': 's',
                    'standard_name':
                    'sea_surface_wave_mean_period_from_variance_spectral_density_inverse_frequency_moment',
                    'description': 'Inverse wave period ((m-1/m0))'
                }),
            'Tp':
            xr.DataArray(
                np.array(Tp),
                dims=['time'],
                attrs={
                    'unit':
                    's',
                    'standard_name':
                    'sea_surface_wave_period_at_variance_spectral_density_maximum',
                    'description':
                    'Peak period (period with maximum elevation energy)'
                }),
            'm_1':
            xr.DataArray(
                np.array(m_1),
                dims=['time'],
                attrs={'description': 'Inverse order moment from spectrum'}),
            'm0':
            xr.DataArray(
                np.array(m0),
                dims=['time'],
                attrs={'description': 'Zeroth order moment from spectrum'}),
            'm1':
            xr.DataArray(
                np.array(m1),
                dims=['time'],
                attrs={'description': 'First order moment from spectrum'}),
            'm2':
            xr.DataArray(
                np.array(m2),
                dims=['time'],
                attrs={'description': 'Second order moment from spectrum'}),
            'm4':
            xr.DataArray(
                np.array(m4),
                dims=['time'],
                attrs={'description': 'Forth order moment from spectrum'}),
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
                            xr.Variable(('time'),
                                        pd.to_datetime(self.time).tz_localize(
                                            None).astype('datetime64[ns]')),
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
