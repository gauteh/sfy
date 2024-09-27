# ifndef GPS
# include "gps.h"

static uint64_t last = 0;
# define RATE 20 // Hz

std::vector<GpsM> msgs;

TwoWire GnssWire(3);
SFE_UBLOX_GNSS gnss;

void getPVT(UBX_NAV_PVT_data_t *ubxDataStruct)
{
    last = micros();
    Serial.print(F("DEMOGPS: New sample, queue: "));
    Serial.println(msgs.size());



    // push new measurement.
    GpsM m;
    m.cputime = micros();
    m.ppsdiff = m.cputime - m.ppsdiff;
    m.gpstime = 0;

    msgs.push_back(m);
}

void setup_gps() {
  Serial.println(F("GPS: Initiating GNSS."));

  GnssWire.begin();
  delay(1000); // Give it time to power up

  if (!gnss.begin(GnssWire, 0x42)){
    Serial.println(F("problem starting GNSS"));

    return;

    delay(500);
  } else{
    Serial.println(F("success starting GNSS"));
  }

  // factory reset (no need to do this every time, only once).
  gnss.factoryDefault();
  delay(5000);

  uint8_t ok = gnss.setI2COutput(COM_TYPE_UBX); //Turn off NMEA noise
  if (ok) ok = gnss.setI2CInput(COM_TYPE_UBX);
  if (ok) ok = gnss.setUART1Output(0);
  if (ok) ok = gnss.setUART1Input(0);
  if (ok) ok = gnss.setNavigationFrequency(1); //Set output in Hz.

  Serial.print(F("GPS setup flag: "));
  Serial.println(ok);

  /* gnss.setAutoPVT(true); */
  gnss.setAutoPVTcallbackPtr(&getPVT); // Enable automatic NAV PVT messages with callback to printPVTdata so we can watch the carrier solution go to fixed
}

void loop_gps() {
  // TODO: move these to PPS?
  gnss.checkUblox();
  gnss.checkCallbacks();
}


void pps() {
  // pps pulse interrupt: should be triggered at the start of every gps second.
  pps_ts = micros();

  Serial.println("GNSS: PPS!");

  // TODO: checkUblox + checkCallbacks
}

# endif
