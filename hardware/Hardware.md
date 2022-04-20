# Hardware

| Component                      | Homepage | Hardware repository | Dimensions |
|--------------------------------|----------|---------------------|------------|
| Sparkfun Redboard Artemis Nano | [sparkfun](https://www.sparkfun.com/products/15443?_ga=2.158373882.1315469777.1632664195-1601050442.1628850831) | [github](https://github.com/sparkfun/RedBoard_Artemis_Nano) | [pdf](https://github.com/sparkfun/RedBoard_Artemis_Nano/blob/master/Documents/Dimensions.pdf) |


# Assembly

* Assemble parts without soldering on first to make sure correct orientation and
    side is used.

## Artemis

1) Solder on pin headers
2) Solder onto protoboard, USB port out.

## Notecard

1) Remove 0Ohm resistor if using passive GPS antenna.
2) Cut all pins, except VIO on VIO side (3V3).
2) Use side with where + rail is outermost for VIO (3V3)
3) Mark this side on protoboard as 3V3, and other as 5V.
4) Make marks where the SDA, SCL and power pins are connected on protoboard

## IMU

1) On VDD, VDDIO side remove all pins except: VDD, VDDIO, SCx, SDx.
2) This side will fit on other side of protoboard, so that VDD, VDDIO, etc are
connected to the VIO (3V3) rail.

## Super-capacitors

1) Solder in series over 5V power-rail.

## Power

1) Attach power-cables with connector to 5V side as close as possible to V+ and
GND of notecard as possible. Make sure this loop uses high-quality wires and
good solders.
