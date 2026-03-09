#! python
#

from subprocess import check_call, Popen, PIPE
from pathlib import Path
import sys
import coloredlogs
import logging

coloredlogs.install()
logger = logging.getLogger("jlink-run")

elf = Path(sys.argv[1])
bin = elf.parent / (elf.name + '.bin')

logger.info(f"Creating binary from elf: {elf} => {bin}..")
check_call(['arm-none-eabi-objcopy', '-S', '-O', 'binary', elf, bin])

logger.info("Starting JLinkExe..")
jlink = Popen(['JLinkExe', '-device', 'AMA3B1KK-KBR', '-autoconnect', '1', '-if', 'swd', '-speed', '4000'])

class Jlink:
    p = None

    def __init__(self):
        pass

    def halt(self):
        pass

