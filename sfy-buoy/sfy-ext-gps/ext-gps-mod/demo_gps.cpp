# ifndef GPS
# include "gps.h"

static uint32_t last = 0;
# define RATE 20 // Hz

std::vector<GpsM> msgs;

void setup_gps() {
  Serial.println(F("GPS: DEMO GPS initiated."));
}

void loop_gps() {
  if (micros() - last > (1000000. / RATE)) {

    pps(); // trigger fake PPS in DEMO
    delay(100); // fake delay between PPS, and read sample

    last = micros();
    Serial.println(F("DEMOGPS: New sample"));

    // push new measurement.
    GpsM m;
    m.cputime = micros();
    m.ppsdiff = m.cputime - m.ppsdiff;
    m.gpstime = 0;

    msgs.push_back(m);

  }
}

void pps() {
  // pps pulse interrupt detected
  pps_ts = micros();
}

# endif
