# include "gps.h"
# include "ArduinoJson.h"
# include "spartn_keys.h"

SFE_UBLOX_GNSS myGNSS; // ZED-F9x
SFE_UBLOX_GNSS myLBand; // NEO-D9S

TwoWire GnssWire(3);

// Serial to SFY
// TX1: ~7 / 42
// RX1: ~8 / 38
// ..
// TX1: ~9 / 39
// RX1: ~10 / 40
Uart sfy{1, 40, 39};

void pushRXMPMP(UBX_RXM_PMP_message_data_t *pmpData)
{
  //Extract the raw message payload length
  uint16_t payloadLen = ((uint16_t)pmpData->lengthMSB << 8) | (uint16_t)pmpData->lengthLSB;
  Serial.print(F("New RXM-PMP data received. Message payload length is "));
  Serial.print(payloadLen);

#ifndef noPush

  Serial.println(F(" Bytes. Pushing it to the GNSS..."));

  //Push the PMP data to the GNSS
  //The payload length could be variable, so we need to push the header and payload, then checksum
  myGNSS.pushRawData(&pmpData->sync1, (size_t)payloadLen + 6); // Push the sync chars, class, ID, length and payload
  myGNSS.pushRawData(&pmpData->checksumA, (size_t)2); // Push the checksum bytes

#else

  Serial.println(F(" Bytes."));

#endif

}

//=-=-=-=-=-=-=-=-=-=-=-=-=-=-=-=-=-=-=-=-=-=-=-=-=-=-=-=-=-=-=-=-=-=-=-=-=-=-=-=-=-=-=-=-=
// Callback: printPVTdata will be called when new NAV PVT data arrives
// See u-blox_structs.h for the full definition of UBX_NAV_PVT_data_t
//         _____  You can use any name you like for the callback. Use the same name when you call setAutoPVTcallbackPtr
//        /                  _____  This _must_ be UBX_NAV_PVT_data_t
//        |                 /               _____ You can use any name you like for the struct
//        |                 |              /
//        |                 |              |
void printPVTdata(UBX_NAV_PVT_data_t *ubxDataStruct)
{
  // Time from GNSS Solution (UTC)
  uint16_t year = ubxDataStruct->year;
  uint8_t month = ubxDataStruct->month;
  uint8_t day = ubxDataStruct->day;
  uint8_t hour = ubxDataStruct->hour;
  uint8_t minute = ubxDataStruct->min;
  uint8_t sec = ubxDataStruct->sec;
  int32_t nano = ubxDataStruct->nano;

  String datetime = String(year) + "-" + String(month) + "-" + String(day) + ":" + String(hour) + ":" + String(minute) + ":" + String(sec) + "." + String(nano);
  Serial.print(F("Time: "));
  Serial.println(datetime);

  uint32_t tAcc = ubxDataStruct->tAcc;
  Serial.print(F("  Time accuracy (ns): "));
  Serial.println(tAcc);

  int32_t latitude = ubxDataStruct->lat; // Print the latitude
  Serial.print(F("  Lat: "));
  Serial.print(latitude / 10000000.0, 7);

  int32_t longitude = ubxDataStruct->lon; // Print the longitude
  Serial.print(F("  Long: "));
  Serial.print(longitude / 10000000.0, 7);

  int32_t altitude = ubxDataStruct->hMSL; // Print the height above mean sea level
  Serial.print(F("  Height: "));
  Serial.print(altitude / 1000.0, 3);

  uint8_t fixType = ubxDataStruct->fixType; // Print the fix type
  Serial.print(F("  Fix: "));
  Serial.print(fixType);
  if (fixType == 0)
    Serial.print(F(" (None)"));
  else if (fixType == 1)
    Serial.print(F(" (Dead Reckoning)"));
  else if (fixType == 2)
    Serial.print(F(" (2D)"));
  else if (fixType == 3)
    Serial.print(F(" (3D)"));
  else if (fixType == 4)
    Serial.print(F(" (GNSS + Dead Reckoning)"));
  else if (fixType == 5)
    Serial.print(F(" (Time Only)"));
  else
    Serial.print(F(" (UNKNOWN)"));

  uint8_t carrSoln = ubxDataStruct->flags.bits.carrSoln; // Print the carrier solution
  Serial.print(F("  Carrier Solution: "));
  Serial.print(carrSoln);
  if (carrSoln == 0)
    Serial.print(F(" (None)"));
  else if (carrSoln == 1)
    Serial.print(F(" (Floating)"));
  else if (carrSoln == 2)
    Serial.print(F(" (Fixed)"));
  else
    Serial.print(F(" (UNKNOWN)"));

  uint32_t hAcc = ubxDataStruct->hAcc; // Print the horizontal accuracy estimate
  Serial.print(F("  Horizontal Accuracy Estimate: "));
  Serial.print(hAcc);
  Serial.print(F(" (mm)"));

  uint32_t vAcc = ubxDataStruct->vAcc; // Print the horizontal accuracy estimate
  Serial.print(F("  Vertical Accuracy Estimate: "));
  Serial.print(vAcc);
  Serial.print(F(" (mm)"));

  Serial.println();

  // Serialize Data and Pipe to SFY
  JsonDocument doc;
  doc["year"] = year;
  doc["month"] = month;
  doc["day"] = day;
  doc["hour"] = hour;
  doc["minute"] = minute;
  doc["sec"] = sec;
  doc["nano"] = nano;
  doc["time_acc"] = tAcc;
  doc["lat"] = latitude;  // 1e7 * deg
  doc["lon"] = longitude; // 1e7 * deg
  doc["msl"] = altitude; // mm
  doc["hor_acc"] = hAcc; // mm
  doc["vert_acc"] = vAcc; // mm
  doc["soln"] = carrSoln;
  doc["fix"] = fixType;
  serializeJson(doc, sfy);
  sfy.println();

  Serial.print("Sent GPS telegram: ");
  serializeJson(doc, Serial);
  Serial.println();
}
//=-=-=-=-=-=-=-=-=-=-=-=-=-=-=-=-=-=-=-=-=-=-=-=-=-=-=-=-=-=-=-=-=-=-=-=-=-=-=-=-=-=-=-=-=
// Callback: printRXMCOR will be called when new RXM COR data arrives
// See u-blox_structs.h for the full definition of UBX_RXM_COR_data_t
//         _____  You can use any name you like for the callback. Use the same name when you call setRXMCORcallbackPtr
//        /                  _____  This _must_ be UBX_RXM_COR_data_t
//        |                 /               _____ You can use any name you like for the struct
//        |                 |              /
//        |                 |              |
void printRXMCOR(UBX_RXM_COR_data_t *ubxDataStruct)
{
  Serial.print(F("UBX-RXM-COR:  ebno: "));
  Serial.print((double)ubxDataStruct->ebno / 8, 3); //Convert ebno to dB

  Serial.print(F("  protocol: "));
  if (ubxDataStruct->statusInfo.bits.protocol == 1)
    Serial.print(F("RTCM3"));
  else if (ubxDataStruct->statusInfo.bits.protocol == 2)
    Serial.print(F("SPARTN"));
  else if (ubxDataStruct->statusInfo.bits.protocol == 29)
    Serial.print(F("PMP (SPARTN)"));
  else if (ubxDataStruct->statusInfo.bits.protocol == 30)
    Serial.print(F("QZSSL6"));
  else
    Serial.print(F("Unknown"));

  Serial.print(F("  errStatus: "));
  if (ubxDataStruct->statusInfo.bits.errStatus == 1)
    Serial.print(F("Error-free"));
  else if (ubxDataStruct->statusInfo.bits.errStatus == 2)
    Serial.print(F("Erroneous"));
  else
    Serial.print(F("Unknown"));

  Serial.print(F("  msgUsed: "));
  if (ubxDataStruct->statusInfo.bits.msgUsed == 1)
    Serial.print(F("Not used"));
  else if (ubxDataStruct->statusInfo.bits.msgUsed == 2)
    Serial.print(F("Used"));
  else
    Serial.print(F("Unknown"));

  Serial.print(F("  msgEncrypted: "));
  if (ubxDataStruct->statusInfo.bits.msgEncrypted == 1)
    Serial.print(F("Not encrypted"));
  else if (ubxDataStruct->statusInfo.bits.msgEncrypted == 2)
    Serial.print(F("Encrypted"));
  else
    Serial.print(F("Unknown"));

  Serial.print(F("  msgDecrypted: "));
  if (ubxDataStruct->statusInfo.bits.msgDecrypted == 1)
    Serial.print(F("Not decrypted"));
  else if (ubxDataStruct->statusInfo.bits.msgDecrypted == 2)
    Serial.print(F("Successfully decrypted"));
  else
    Serial.print(F("Unknown"));

  Serial.println();
}


void pps() {
  // pps pulse interrupt: should be triggered at the start of every gps second.
  // pps_ts = micros();

  Serial.println("GNSS: PPS!");
  myGNSS.checkUblox(); // Check for the arrival of new GNSS data and process it.
  myGNSS.checkCallbacks(); // Check if any GNSS callbacks are waiting to be processed.

  myLBand.checkUblox(); // Check for the arrival of new PMP data and process it.
  myLBand.checkCallbacks(); // Check if any LBand callbacks are waiting to be processed.

}

void setup_gps() {

  sfy.begin(400000);

  // serialGNSS.begin(38400); // UART2 on pins 16/17 for SPP. The ZED-F9P will be configured to use the same rate.
  // Wire.begin(); //Start I2C
  GnssWire.begin();

  //=-=-=-=-=-=-=-=-=-=-=-=-=-=-=-=-=-=-=-=-=-=-=-=-=-=-=-=-=-=-=-=-=-=-=-=-=-=-=-=-=-=-=-=-=
  // Begin and configure the ZED-F9x

  // myGNSS.enableDebugging(); // Uncomment this line to enable helpful debug messages on Serial

  while (myGNSS.begin(GnssWire) == false) //Connect to the u-blox module using Wire port
  {
    Serial.println(F("u-blox GNSS module not detected at default I2C address. Please check wiring."));
    delay(2000);
  }
  Serial.println(F("u-blox GNSS module connected"));

  //Check the ZED firmware version - SPARTN is only supported on ZED-F9P from HPG 1.30 and ZED-F9R from HPS 1.21 onwards
  if (myGNSS.getModuleInfo())
  {
    Serial.print(F("FWVER: "));
    Serial.print(myGNSS.getFirmwareVersionHigh()); // Returns uint8_t
    Serial.print(F("."));
    Serial.println(myGNSS.getFirmwareVersionLow()); // Returns uint8_t

    Serial.print(F("Firmware: "));
    Serial.println(myGNSS.getFirmwareType()); // Returns HPG, SPG etc. as (const char *)

    if (strcmp(myGNSS.getFirmwareType(), "HPG") == 0)
      if ((myGNSS.getFirmwareVersionHigh() == 1) && (myGNSS.getFirmwareVersionLow() < 30))
        Serial.println("Your module is running old firmware which may not support SPARTN. Please upgrade.");

    if (strcmp(myGNSS.getFirmwareType(), "HPS") == 0)
      if ((myGNSS.getFirmwareVersionHigh() == 1) && (myGNSS.getFirmwareVersionLow() < 21))
        Serial.println("Your module is running old firmware which may not support SPARTN. Please upgrade.");
  }
  else
    Serial.println(F("Error: could not read module info!"));

  uint8_t ok = myGNSS.setI2COutput(COM_TYPE_UBX); //Turn off NMEA noise
  Serial.println(OK(ok));
  ok = myGNSS.setI2CInput(COM_TYPE_UBX);
  Serial.println(OK(ok));
  ok = myGNSS.setUART1Output(0);
  Serial.println(OK(ok));
  ok = myGNSS.setUART1Input(0);
  Serial.println(OK(ok));
  ok = myGNSS.setUART2Output(0);
  Serial.println(OK(ok));
  ok = myGNSS.setUART2Input(COM_TYPE_UBX | COM_TYPE_NMEA | COM_TYPE_SPARTN); //Be sure SPARTN input is enabled
  Serial.println(OK(ok));
  ok = myGNSS.setDGNSSConfiguration(SFE_UBLOX_DGNSS_MODE_FIXED); // Set the differential mode - ambiguities are fixed whenever possible
  Serial.println(OK(ok));
  ok = myGNSS.setNavigationFrequency(20); //Set output in Hz.
  Serial.println(OK(ok));
  ok = myGNSS.setVal8(UBLOX_CFG_SPARTN_USE_SOURCE, 1); // Set to 1 for using L-Band Correction
  Serial.println(OK(ok));
  ok = myGNSS.setVal8(UBLOX_CFG_MSGOUT_UBX_RXM_COR_I2C, 1); // Enable UBX-RXM-COR messages on I2C
  Serial.println(OK(ok));
  ok = myGNSS.setVal8(UBX_NAV_PVT, 1);

  // Configure the Timing Pulse settings
  myGNSS.newCfgValset(VAL_LAYER_RAM); // Create a new Configuration Interface VALSET message. Apply the changes in RAM only (not BBR).
  // Let's say that we want our 1 pulse every 30 seconds to be as accurate as possible. So, let's tell the module
  // to generate no signal while it is _locking_ to GNSS time. We want the signal to start only when the module is
  // _locked_ to GNSS time.
  myGNSS.addCfgValset(UBLOX_CFG_TP_PERIOD_TP1, 0);
  myGNSS.addCfgValset(UBLOX_CFG_TP_LEN_TP1, 0); // Set the pulse length to zero

  // When the module is _locked_ to GNSS time, make it generate a  second pulse every 30 seconds
  // myGNSS.addCfgValset(UBLOX_CFG_TP_PERIOD_LOCK_TP1, 50000); // Set the period to 30,000,000 us
  // myGNSS.addCfgValset(UBLOX_CFG_TP_LEN_LOCK_TP1, 10000); // Set the pulse length to 1,000,000 us
  myGNSS.addCfgValset(UBLOX_CFG_TP_PERIOD_LOCK_TP1, 1000000); // Set the period to 30,000,000 us
  myGNSS.addCfgValset(UBLOX_CFG_TP_LEN_LOCK_TP1, 100000); // Set the pulse length to 1,000,000 us

  // Now set the time pulse parameters
  if (myGNSS.sendCfgValset() == false)
  {
    Serial.println(F("VALSET failed!"));
  }
  else
  {
    Serial.println(F("Success!"));
  }

  //Configure the SPARTN IP Dynamic Keys
  //"When the receiver boots, the host should send 'current' and 'next' keys in one message." - Use setDynamicSPARTNKeys for this.
  //"Every time the 'current' key is expired, 'next' takes its place."
  //"Therefore the host should then retrieve the new 'next' key and send only that." - Use setDynamicSPARTNKey for this.
  // The key can be provided in binary (uint8_t) format or in ASCII Hex (char) format, but in both cases keyLengthBytes _must_ represent the binary key length in bytes.
  ok = myGNSS.setDynamicSPARTNKeys(currentKeyLengthBytes, currentKeyGPSWeek, currentKeyGPSToW, currentDynamicKey,
                                           nextKeyLengthBytes, nextKeyGPSWeek, nextKeyGPSToW, nextDynamicKey);
  Serial.println(OK(ok));
  //if (ok) ok = myGNSS.saveConfiguration(VAL_CFG_SUBSEC_IOPORT | VAL_CFG_SUBSEC_MSGCONF); //Optional: Save the ioPort and message settings to NVM and BBR

  Serial.print(F("GNSS: configuration "));
  Serial.println(OK(ok));

  myGNSS.setAutoPVTcallbackPtr(&printPVTdata); // Enable automatic NAV PVT messages with callback to printPVTdata so we can watch the carrier solution go to fixed

  myGNSS.setRXMCORcallbackPtr(&printRXMCOR); // Print the contents of UBX-RXM-COR messages so we can check if the PMP data is being decrypted successfully

  //=-=-=-=-=-=-=-=-=-=-=-=-=-=-=-=-=-=-=-=-=-=-=-=-=-=-=-=-=-=-=-=-=-=-=-=-=-=-=-=-=-=-=-=-=
  // Begin and configure the NEO-D9S L-Band receiver

  //myLBand.enableDebugging(); // Uncomment this line to enable helpful debug messages on Serial

  while (myLBand.begin(GnssWire, 0x43) == false) //Connect to the u-blox NEO-D9S using Wire port. The D9S default I2C address is 0x43 (not 0x42)
  {
    Serial.println(F("u-blox NEO-D9S not detected at default I2C address. Please check wiring."));
    delay(2000);
  }

  // L-Band center frequency
  //const uint32_t myLBandFreq = 1556290000; // Uncomment this line to use the US SPARTN 1.8 service
  const uint32_t myLBandFreq = 1545260000; // Uncomment this line to use the EU SPARTN 1.8 service

  // NEO-D9S Configuration Settings
  Serial.println(F("u-blox NEO-D9S connected"));

  myLBand.newCfgValset(); // Create a new Configuration Interface message - this defaults to VAL_LAYER_RAM_BBR (change in RAM and BBR)
  myLBand.addCfgValset(UBLOX_CFG_PMP_CENTER_FREQUENCY,     myLBandFreq); // Default 1539812500 Hz
  myLBand.addCfgValset(UBLOX_CFG_PMP_SEARCH_WINDOW,        2200);        // Default 2200 Hz
  myLBand.addCfgValset(UBLOX_CFG_PMP_USE_SERVICE_ID,       1);           // Default 1
  myLBand.addCfgValset(UBLOX_CFG_PMP_SERVICE_ID,           21845);       // Default 50821
  myLBand.addCfgValset(UBLOX_CFG_PMP_DATA_RATE,            2400);        // Default 2400 bps
  myLBand.addCfgValset(UBLOX_CFG_PMP_USE_DESCRAMBLER,      1);           // Default 1
  myLBand.addCfgValset(UBLOX_CFG_PMP_DESCRAMBLER_INIT,     26969);       // Default 23560
  myLBand.addCfgValset(UBLOX_CFG_PMP_USE_PRESCRAMBLING,    0);           // Default 0
  myLBand.addCfgValset(UBLOX_CFG_PMP_UNIQUE_WORD,          16238547128276412563ull);
  myLBand.addCfgValset(UBLOX_CFG_UART2OUTPROT_UBX,         1);           // Enable UBX output on UART2
  myLBand.addCfgValset(UBLOX_CFG_MSGOUT_UBX_RXM_PMP_UART2, 1);           // Output UBX-RXM-PMP on UART2
  myLBand.addCfgValset(UBLOX_CFG_UART2_BAUDRATE,           38400);       // match baudrate with ZED default
  ok = myLBand.sendCfgValset(); // Apply the settings

  Serial.print(F("L-Band: configuration "));
  Serial.println(OK(ok));

  myLBand.softwareResetGNSSOnly(); // Do a restart

  myLBand.setRXMPMPmessageCallbackPtr(&pushRXMPMP); // Call pushRXMPMP when new PMP data arrives. Push it to the GNSS
}

void loop_gps() {
  // myGNSS.checkUblox(); // Check for the arrival of new GNSS data and process it.
  // myGNSS.checkCallbacks(); // Check if any GNSS callbacks are waiting to be processed.

  // myLBand.checkUblox(); // Check for the arrival of new PMP data and process it.
  // myLBand.checkCallbacks(); // Check if any LBand callbacks are waiting to be processed.
}
