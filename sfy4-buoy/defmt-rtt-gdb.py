#! python
#
# Author: Gaute Hope (eg@gaute.vetsj.com) (c) 2021-11-16
#
# Requirements:
# * nc
# * defmt-print
#
# This command attached to JLinkGDBServer for RTT output and passes that through defmt-print.
#
# Source this file in gdb or .gdbinit and run "defmt-rtt", continue your program with 'c &'.

from threading import Thread
from subprocess import Popen, PIPE, DEVNULL
import os


class DefmtRtt(gdb.Command):
    rtt_th = None

    def __init__(self):
        super().__init__("defmt-rtt", gdb.COMMAND_USER)

    def get_exe(self):
        return gdb.current_progspace().filename

    def invoke(self, arg, from_tty):
        self.dont_repeat()
        exe = self.get_exe()

        gdb.events.exited.connect(self.stop)
        # gdb.events.stop.connect(self.stop)
        gdb.events.cont.connect(self.cont)

        self.rtt_th = DefmtPrinter(exe)
        self.rtt_th.start()

    def stop(self, event):
        self.rtt_th.stop()

    def cont(self, event):
        if not self.rtt_th.is_alive():
            self.rtt_th = DefmtPrinter(self.get_exe())
            self.rtt_th.start()


class DefmtPrinter(Thread):
    exe = None
    prc1 = None
    prc2 = None

    def __init__(self, exe):
        super().__init__()
        self.exe = exe

    def run(self):
        print(f"Attaching to GDB server for RTT | defmt output (exe: {self.exe}..")
        self.prc1 = Popen(["nc", "localhost", "19021"],
                          bufsize=0,
                          stdin=DEVNULL,
                          stdout=PIPE)

        # read jlink header
        for _ in range(0, 3):
            self.prc1.stdout.readline()

        self.prc2 = Popen(["defmt-print", "-e", self.exe],
                          stdin=self.prc1.stdout,
                          bufsize=0)

        self.prc2.wait()
        self.prc1.wait()

    def stop(self):
        # stop the defmt thread
        self.prc2.kill()
        self.prc1.kill()
        self.prc2.wait()
        self.prc1.wait()


# Instantiate so that gdb knows about us.
DefmtRtt()
