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

    def to_nc(self, filename: Path):
        """
        Write a CF-compliant NetCDF file to filename.
        """
        pass

