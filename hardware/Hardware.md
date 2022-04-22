# Hardware

schematic: [(pdf)](sfy-schema-v1.pdf) | [(kicad)](sfy/)

<a href="sfy-schema-v1.pdf">
    <img src="sfy-schema-v1.png" width="100%" />
</a>


# Assembly

* Assemble parts without soldering on first to make sure correct orientation and
    side is used.
* Make sure notecard and artemis firmware is updated.
* Avoid poor jumper-cables.
* When proto-typing on breadboard: many connections and long distances can break
    the setup. Connect power-loop to notecard as close as possible.
* Using different power loops for notecard and rest of components requires high
    quality conductors, otherwise the 2A surges for the GSM modem will cause
    significant voltage shifts between the loops.

## Artemis

1) Solder on pin headers
2) Solder onto protoboard, USB port out.
3) Program artemis with firmware

## Notecard

1) Remove 0Ohm resistor if using passive GPS antenna (usually that is the case
for our application).
2) Cut all pins, except VIO on VIO side (3V3).
2) Use side with where + rail is outermost for VIO (3V3)
3) Mark this side on protoboard as 3V3, and other as 5V.
4) Make marks where the SDA, SCL and power pins are connected on protoboard
5) Update Notecard firmware.

## IMU

1) On VDD, VDDIO side remove all pins except: VDD, VDDIO, SCx, SDx.
2) This side will fit on other side of protoboard, so that VDD, VDDIO, etc are
connected to the VIO (3V3) rail.

## Super-capacitors

1) Solder in series over 5V power-rail.

## Power

1) Attach power-cables with connector to 5V side as close as possible to V+ and
GND of notecard as possible. Make sure this loop uses high-quality wires and
is soldered well.
