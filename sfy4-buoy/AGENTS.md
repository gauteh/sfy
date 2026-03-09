# Main goal

Update the sfy3-buoy/sfy-artemis implementation to use the newly added max-m10s
gps.

Resources:
* The max-m10s driver is located at: https://github.com/gauteh/max-m10s-rs
* The schema for the hardware is located at ../hardware/v4.0.0/schema.pdf. For
    the gps the wiring is the same as the examples in the max-m10s-rs
    repository.

# Details

* The old version is located in sfy3-buoy, the new version you should create
    goes in sfy4-buoy. It is currently almost empty. The old version may be used
    as a template.
* The library in sfy4-buoy may be simplified to no longer match legacy code.
* the ext-gps feature can be removed (essentially always enabled), as the max-m10s is always mounted.
* Remove the JSON interface to the existing gps, and use the read_pvt directly.
* Update the GPIO interrupt to set the time using the new interface.

* Use the local copy of the driver in ../../max-m10s-rs for development.
* If features are missing in the max-m10s-rs driver they can be requested from
    agent in session 'max-m10s'.

* use `make host-tests` to run tests.

## Future work (not for now)

* Store gps to SD card as well.
* Make duty-cycle of gps configurable:
    - Turn with a certain delay.
    - Collect data for 20 minutes.
* Use GPS velocity to calculate a separate spectrum, like it is currently done
    for the IMU in spec. Create a separate package type.
