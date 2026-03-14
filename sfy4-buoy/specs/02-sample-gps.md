# Main goal

After ./01-impl-gps-driver.md has been implemented. It is now possible to read the gps. However, it is only reading it in the main loop. Keep those in mind as well.

* The GPS should now be collected in the GPS GPIO interrupt, so that every time a time pulse is sent a new sample is read and pushed in to the collector.
* It may be possible to configure the MAX-M10S to send an interrupt when a new sample is ready.
* Make sure the location and RTC is still set every second.

# General skills.

* Make sure all code builds and tests.
* Keep things simple if possible.
