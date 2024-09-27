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

    // power things down
    delay(500);
  } else{
    Serial.println(F("success starting GNSS"));
  }

  uint8_t ok = gnss.setI2COutput(COM_TYPE_UBX); //Turn off NMEA noise
  if (ok) ok = gnss.setI2CInput(COM_TYPE_UBX);
  if (ok) ok = gnss.setUART1Output(0);
  if (ok) ok = gnss.setUART1Input(0);
  if (ok) ok = gnss.setUART2Output(0);
  /* if (ok) ok = gnss.setUART2Input(COM_TYPE_SPARTN); //Be sure SPARTN input is enabled */
  /* if (ok) ok = gnss.setDGNSSConfiguration(SFE_UBLOX_DGNSS_MODE_FIXED); // Set the differential mode - ambiguities are fixed whenever possible */
  if (ok) ok = gnss.setNavigationFrequency(1); //Set output in Hz.
  /* if (ok) ok = myGNSS.setVal8(UBLOX_CFG_SPARTN_USE_SOURCE, 0); // Use "IP" source, not L-Band. We are injecting raw SPARTN, not PMP */
  /* if (ok) ok = myGNSS.setVal8(UBLOX_CFG_MSGOUT_UBX_RXM_COR_I2C, 1); // Enable UBX-RXM-COR messages on I2C */

  // now we know that we can talk to the gnss
  /* delay(100); */

  // If we are going to change the dynamic platform model, let's do it here.
  // Possible values are:
  // PORTABLE,STATIONARY,PEDESTRIAN,AUTOMOTIVE,SEA,AIRBORNE1g,AIRBORNE2g,AIRBORNE4g,WRIST,BIKE
  /* if (!gnss.setDynamicModel(DYN_MODEL_STATIONARY)){ */
  /*   Serial.println(F("GNSS could not set dynamic model")); */
  /* } */

  /* gnss.setAutoPVT(true); */
  gnss.setAutoPVTcallbackPtr(&getPVT); // Enable automatic NAV PVT messages with callback to printPVTdata so we can watch the carrier solution go to fixed
}

void loop_gps() {
  gnss.checkUblox();
  gnss.checkCallbacks();
}


void pps() {
  // pps pulse interrupt: should be triggered at the start of every gps second.
  pps_ts = micros();
}

# endif
