import numpy as np
import matplotlib.pyplot as plt
import scipy as sc, scipy.fft
from sfy import axl
import pytest

@pytest.fixture
def signal():
    d = open(
        'tests/data/dev864475044203262/1639855192872-3a0c5fc2-e79f-48d1-91e9-e104ac937644_axl.qo.json'
    ).read()
    a = axl.Axl.parse(d)

    # s = np.empty((1024*3,), np.float16)

    # s[0::3] = a.x
    # s[1::3] = a.y
    # s[2::3] = a.z

    # s[:1024] = a.x
    # s[1024:2*1024] = a.y
    # s[2*1024:3*1024] = a.z

    return a.x

def test_compress_deflate(signal):
    import zlib

    z = signal.tobytes()
    jz = zlib.compress(z, 9)

    print(f"compressed: z={len(z)} -> jz={len(jz)}")

def test_compress_zlib_dct(signal):
    import zlib

    z = signal
    Z = sc.fft.dct(z, type = 4, norm='forward')
    q = 1./(np.arange(1, len(Z)+1))
    Z = Z*q
    Z[np.abs(Z)<.0001] = 0
    # plt.plot(Z)
    # plt.show()

    jz = zlib.compress(Z.astype(np.float16).tobytes(), 9)

    print(f"compressed: z={len(signal.tobytes())} -> Z={len(Z.tobytes())} -> jz={len(jz)}")


def test_compress_lzma(signal):
    import lzma

    z = signal
    # Z = sc.fft.dct(z, type = 4).astype(np.float16)
    # Z[np.abs(Z)<.1] = 0
    # Z = np.diff(z)
    Z = z
    jz = lzma.compress(Z.tobytes())

    print(f"lzma: compressed: z={len(signal.tobytes())} -> Z={len(Z.tobytes())} -> jz={len(jz)}")


